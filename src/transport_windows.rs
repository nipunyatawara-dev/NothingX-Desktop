//! Native Bluetooth Classic RFCOMM. All sockets belong to one worker;
//! Winsock events wake it for network data, writes and shutdown (no polling).
use super::*;
use std::{
    collections::VecDeque,
    mem::size_of,
    ptr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
};
use windows_sys::{
    Win32::{Devices::Bluetooth::*, Networking::WinSock::*},
    core::GUID,
};

pub struct Transport {
    tx: Sender<Vec<u8>>,
    wake: WSAEVENT,
    stopping: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
struct Socket(SOCKET);
impl Drop for Socket {
    fn drop(&mut self) {
        unsafe {
            closesocket(self.0);
        }
    }
}
struct NetEvent(WSAEVENT);
impl Drop for NetEvent {
    fn drop(&mut self) {
        unsafe {
            WSACloseEvent(self.0);
        }
    }
}

impl Transport {
    pub fn start(proxy: EventLoopProxy<Event>) -> Result<Self, String> {
        // SAFETY: valid, initialized Winsock storage; lifetime ends after worker join.
        unsafe {
            let mut data = WSADATA::default();
            let error = WSAStartup(0x0202, &mut data);
            if error != 0 {
                return Err(format!("Windows Bluetooth startup failed ({error})"));
            }
            let wake = WSACreateEvent();
            if wake == 0 {
                WSACleanup();
                return Err("Could not create Bluetooth command event".into());
            }
            let (tx, rx) = mpsc::channel();
            let stopping = Arc::new(AtomicBool::new(false));
            let stop = stopping.clone();
            let worker = thread::spawn(move || run(proxy, rx, wake, stop));
            Ok(Self {
                tx,
                wake,
                stopping,
                worker: Some(worker),
            })
        }
    }
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.tx.send(bytes.to_vec()).map_err(|e| e.to_string())?;
        unsafe {
            WSASetEvent(self.wake);
        }
        Ok(())
    }
    pub fn refresh(&mut self) {
        unsafe {
            WSASetEvent(self.wake);
        }
    }
    pub fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            self.stopping.store(true, Ordering::Release);
            unsafe {
                WSASetEvent(self.wake);
            }
            let _ = worker.join();
            unsafe {
                WSACloseEvent(self.wake);
                WSACleanup();
            }
        }
    }
}
impl Drop for Transport {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(proxy: EventLoopProxy<Event>, rx: Receiver<Vec<u8>>, wake: WSAEVENT, stop: Arc<AtomicBool>) {
    let mut retry = 3_000;
    while !stop.load(Ordering::Acquire) {
        for (name, address) in candidates() {
            if stop.load(Ordering::Acquire) {
                return;
            }
            let Some((socket, event)) = open(address, wake, &stop) else {
                continue;
            };
            // Commands from a previous connection may never leak into a new one.
            while rx.try_recv().is_ok() {}
            if proxy
                .send_event(Event::Connected(name, format!("{address:012X}")))
                .is_err()
            {
                return;
            }
            connected(&socket, &event, wake, &rx, &stop, &proxy);
            let _ = proxy.send_event(Event::Disconnected);
            retry = 3_000;
            break;
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
        // Discard writes while disconnected. Manual-reset command event also cancels retries.
        unsafe {
            WSAResetEvent(wake);
        }
        while rx.try_recv().is_ok() {}
        if stop.load(Ordering::Acquire) {
            break;
        }
        unsafe {
            WSAWaitForMultipleEvents(1, &(wake as *mut core::ffi::c_void), 0, retry, 0);
        }
        retry = (retry * 2).min(30_000);
    }
}

fn candidates() -> Vec<(String, u64)> {
    let mut result = Vec::new();
    // SAFETY: structure sizes follow the Windows ABI; no inquiry or pairing changes.
    unsafe {
        let search = BLUETOOTH_DEVICE_SEARCH_PARAMS {
            dwSize: size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
            fReturnAuthenticated: 1,
            fReturnRemembered: 1,
            fReturnConnected: 1,
            ..Default::default()
        };
        let mut d = BLUETOOTH_DEVICE_INFO {
            dwSize: size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
            ..Default::default()
        };
        let find = BluetoothFindFirstDevice(&search, &mut d);
        if find.is_null() {
            return result;
        }
        loop {
            let end = d
                .szName
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(d.szName.len());
            let name = String::from_utf16_lossy(&d.szName[..end]);
            if d.fConnected != 0
                && (nothingx_desktop::models::find(&name).is_some()
                    || d.ulClassofDevice & 0x1f00 == 0x0400)
            {
                result.push((name, d.Address.Anonymous.ullLong));
            }
            d.dwSize = size_of::<BLUETOOTH_DEVICE_INFO>() as u32;
            if BluetoothFindNextDevice(find, &mut d) == 0 {
                break;
            }
        }
        BluetoothFindDeviceClose(find);
    }
    result
}

fn open(address: u64, wake: WSAEVENT, stop: &AtomicBool) -> Option<(Socket, NetEvent)> {
    // SAFETY: sockets and network events are RAII-owned. SOCKADDR_BTH is passed
    // at its exact packed ABI size; UUID resolution is performed by Windows SDP.
    unsafe {
        let raw = socket(i32::from(AF_BTH), SOCK_STREAM, BTHPROTO_RFCOMM as i32);
        if raw == INVALID_SOCKET {
            return None;
        }
        let socket = Socket(raw);
        let raw_event = WSACreateEvent();
        if raw_event == 0 {
            return None;
        }
        let event = NetEvent(raw_event);
        if WSAEventSelect(raw, raw_event, FD_CONNECT as i32) != 0 {
            return None;
        }
        let remote = SOCKADDR_BTH {
            addressFamily: AF_BTH,
            btAddr: address,
            serviceClassId: GUID::from_u128(0xaeac4a03_dff5_498f_843a_34487cf133eb),
            port: 0,
        };
        if connect(
            raw,
            ptr::from_ref(&remote).cast(),
            size_of::<SOCKADDR_BTH>() as i32,
        ) != 0
        {
            if WSAGetLastError() != WSAEWOULDBLOCK {
                return None;
            }
            let events = [
                wake as *mut core::ffi::c_void,
                raw_event as *mut core::ffi::c_void,
            ];
            let selected = WSAWaitForMultipleEvents(2, events.as_ptr(), 0, 8_000, 0);
            if selected != 1 || stop.load(Ordering::Acquire) {
                return None;
            }
            let mut network = WSANETWORKEVENTS::default();
            if WSAEnumNetworkEvents(raw, raw_event, &mut network) != 0
                || network.iErrorCode[FD_CONNECT_BIT as usize] != 0
            {
                return None;
            }
        }
        if WSAEventSelect(raw, raw_event, (FD_READ | FD_WRITE | FD_CLOSE) as i32) != 0 {
            return None;
        }
        Some((socket, event))
    }
}

fn connected(
    socket: &Socket,
    event: &NetEvent,
    wake: WSAEVENT,
    rx: &Receiver<Vec<u8>>,
    stop: &AtomicBool,
    proxy: &EventLoopProxy<Event>,
) {
    let mut output = VecDeque::<u8>::new();
    let mut bytes = [0u8; 4096];
    loop {
        if stop.load(Ordering::Acquire) {
            return;
        }
        let handles = [
            wake as *mut core::ffi::c_void,
            event.0 as *mut core::ffi::c_void,
        ];
        let selected = unsafe { WSAWaitForMultipleEvents(2, handles.as_ptr(), 0, WSA_INFINITE, 0) };
        if selected == WSA_WAIT_FAILED {
            return;
        }
        unsafe {
            WSAResetEvent(wake);
        }
        while let Ok(frame) = rx.try_recv() {
            output.extend(frame);
        }
        if stop.load(Ordering::Acquire) {
            return;
        }
        let mut network = WSANETWORKEVENTS::default();
        if unsafe { WSAEnumNetworkEvents(socket.0, event.0, &mut network) } != 0 {
            return;
        }
        // Drain all available bytes; reading re-arms FD_READ.
        loop {
            let count = unsafe { recv(socket.0, bytes.as_mut_ptr(), bytes.len() as i32, 0) };
            if count == 0 {
                return;
            }
            if count < 0 {
                if unsafe { WSAGetLastError() } == WSAEWOULDBLOCK {
                    break;
                }
                return;
            }
            if proxy
                .send_event(Event::Data(bytes[..count as usize].to_vec()))
                .is_err()
            {
                return;
            }
        }
        if network.lNetworkEvents & FD_CLOSE as i32 != 0 {
            return;
        }
        while !output.is_empty() {
            let part = output.make_contiguous();
            let count = unsafe { send(socket.0, part.as_ptr(), part.len() as i32, 0) };
            if count < 0 {
                if unsafe { WSAGetLastError() } == WSAEWOULDBLOCK {
                    break;
                }
                return;
            }
            if count == 0 {
                return;
            }
            output.drain(..count as usize);
        }
    }
}
