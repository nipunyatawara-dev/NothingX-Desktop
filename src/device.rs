//! Single-thread-owned device state and deterministic command scheduling.
use crate::{
    models::{self, Capabilities},
    protocol::{self, Frame, Parser},
};
use std::{
    collections::{BTreeMap, VecDeque},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    Noise,
    Bass,
    InEar,
    LowLatency,
    Personal,
    Spatial,
    Preset,
    CustomEq,
    Dual,
    Gesture(u8, u8),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Byte(u8),
    Flag(bool),
    Pair(u8, u8),
    Eq([i8; 3]),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Battery {
    pub level: u8,
    pub charging: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DualDevice {
    pub address: [u8; 6],
    pub name: String,
    pub is_self: bool,
    pub connected: bool,
}
#[derive(Clone, Debug)]
pub enum Action {
    Set(Key, Value),
    Refresh,
    Ring(u8),
    StopRing,
    DualConnect([u8; 6], bool),
}

struct Pending {
    target: Value,
    next: Instant,
    deadline: Instant,
}
pub struct Device {
    pub name: String,
    pub address: String,
    pub connected: bool,
    pub caps: Option<Capabilities>,
    pub battery: BTreeMap<u8, Battery>,
    pub values: BTreeMap<Key, Value>,
    pub firmware: String,
    pub devices: Vec<DualDevice>,
    pub message: String,
    pub ringing: Option<u8>,
    pub dual_busy: Option<([u8; 6], Instant)>,
    pub simulation: bool,
    parser: Parser,
    operation: u8,
    queue: VecDeque<(u16, Vec<u8>)>,
    pending: BTreeMap<Key, Pending>,
    next_send: Instant,
    next_battery: Instant,
    ring_stop: Option<Instant>,
    dual_pages: u8,
}

impl Default for Device {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            name: String::new(),
            address: String::new(),
            connected: false,
            caps: None,
            battery: BTreeMap::new(),
            values: BTreeMap::new(),
            firmware: String::new(),
            devices: Vec::new(),
            message: "Pair and connect your headphones in Bluetooth settings.".into(),
            ringing: None,
            dual_busy: None,
            simulation: false,
            parser: Parser::default(),
            operation: 0,
            queue: VecDeque::new(),
            pending: BTreeMap::new(),
            next_send: now,
            next_battery: now,
            ring_stop: None,
            dual_pages: 0,
        }
    }
}

impl Device {
    pub fn on_connected(&mut self, name: String, address: String) {
        *self = Self::default();
        self.caps = models::find(&name);
        self.name = name;
        self.address = address;
        self.connected = true;
        self.message = if self.caps.is_some() {
            "Reading device settings…"
        } else {
            "Unknown model: battery and firmware only."
        }
        .into();
        self.query_all();
        self.next_battery = Instant::now() + Duration::from_secs(60);
    }
    pub fn disconnect(&mut self) {
        *self = Self::default();
        self.message = "Disconnected. Waiting for a connected Nothing or CMF device…".into();
    }
    pub fn simulate(name: &str) -> Self {
        let mut d = Self::default();
        d.on_connected(name.into(), "Simulation".into());
        d.simulation = true;
        d.message = "SIMULATION — no Bluetooth connection".into();
        d.queue.clear();
        d.battery = BTreeMap::from([
            (
                2,
                Battery {
                    level: 87,
                    charging: false,
                },
            ),
            (
                3,
                Battery {
                    level: 80,
                    charging: false,
                },
            ),
            (
                4,
                Battery {
                    level: 62,
                    charging: true,
                },
            ),
        ]);
        d.values = BTreeMap::from([
            (Key::Noise, Value::Byte(1)),
            (Key::Bass, Value::Pair(1, 6)),
            (Key::InEar, Value::Flag(true)),
            (Key::LowLatency, Value::Flag(false)),
            (Key::Personal, Value::Flag(false)),
            (Key::Spatial, Value::Pair(0, 0)),
            (Key::Preset, Value::Byte(3)),
            (Key::CustomEq, Value::Eq([0, 0, 0])),
            (Key::Dual, Value::Flag(true)),
        ]);
        d.firmware = "Simulation".into();
        d.name = format!("{name} · Simulation");
        d.devices = vec![
            DualDevice {
                address: [1, 2, 3, 4, 5, 6],
                name: "This computer".into(),
                is_self: true,
                connected: true,
            },
            DualDevice {
                address: [6, 5, 4, 3, 2, 1],
                name: "Phone (demo)".into(),
                is_self: false,
                connected: false,
            },
        ];
        for side in [2, 3] {
            for (slot, action) in [(2, 2), (3, 9), (7, 22), (9, 18)] {
                d.values
                    .insert(Key::Gesture(side, slot), Value::Byte(action));
            }
        }
        d
    }
    pub fn is_pending(&self, key: Key) -> bool {
        self.pending.contains_key(&key)
    }
    pub fn value(&self, key: Key) -> Option<&Value> {
        self.values.get(&key)
    }
    pub fn flag(&self, key: Key) -> bool {
        self.value(key) == Some(&Value::Flag(true))
    }

    fn enqueue(&mut self, command: u16, payload: Vec<u8>) {
        if self.connected && !self.simulation && self.queue.len() < 64 {
            self.queue.push_back((command, payload));
        }
    }
    fn read(&mut self, key: Key) {
        if let Some((get, _, _)) = self.spec(key) {
            self.enqueue(get, vec![]);
        }
    }
    fn query_all(&mut self) {
        self.enqueue(0xc007, vec![]);
        self.enqueue(0xc042, vec![]);
        for key in [
            Key::Noise,
            Key::Bass,
            Key::InEar,
            Key::LowLatency,
            Key::Personal,
            Key::Spatial,
            Key::Preset,
            Key::CustomEq,
            Key::Dual,
        ] {
            self.read(key);
        }
        if self.caps.as_ref().is_some_and(|c| c.gestures.is_some()) {
            self.enqueue(0xc018, vec![]);
        }
        if self.caps.as_ref().is_some_and(|c| c.dual_connection) {
            self.refresh_dual();
        }
    }
    fn refresh_dual(&mut self) {
        self.devices.clear();
        self.dual_pages = 1;
        self.enqueue(0xc028, vec![0]);
    }

    /// (read command, write command, supported). No guessed writes for unknown models.
    fn spec(&self, key: Key) -> Option<(u16, u16, bool)> {
        let c = self.caps.as_ref()?;
        let spec = match key {
            Key::Noise => (0xc01e, 0xf00f, c.noise_cancellation || c.transparency),
            Key::Bass => (0xc04e, 0xf051, c.bass_steps > 0),
            Key::InEar => (0xc00e, 0xf004, c.in_ear),
            Key::LowLatency => (0xc041, 0xf040, c.low_latency),
            Key::Personal => match c.personal_sound.as_deref() {
                Some("mimi") => (0xc022, 0xf015, true),
                Some("audiodo") => (0xc05a, 0xf05c, true),
                _ => return None,
            },
            Key::Spatial => (0xc04f, 0xf052, !c.spatial_modes.is_empty()),
            Key::Preset => {
                if c.eq.style == "listening" {
                    (0xc050, 0xf01d, true)
                } else {
                    (0xc01f, 0xf010, true)
                }
            }
            Key::CustomEq => (0xc044, 0xf041, c.eq.custom_id.is_some()),
            Key::Dual => (0xc027, 0xf01a, c.dual_connection),
            Key::Gesture(side, slot) => (
                0xc018,
                0xf003,
                [2, 3].contains(&side)
                    && c.gestures
                        .as_ref()
                        .is_some_and(|p| p.slots.iter().any(|s| s.r#type == slot)),
            ),
        };
        spec.2.then_some(spec)
    }

    fn payload(&self, key: Key, value: &Value) -> Option<Vec<u8>> {
        let c = self.caps.as_ref()?;
        match (key, value) {
            (Key::Noise, Value::Byte(code)) => {
                let valid = match code {
                    5 => true,
                    7 => c.transparency,
                    1 => c.noise_cancellation,
                    2 => c.strengths.iter().any(|s| s == "mid"),
                    3 => c.strengths.iter().any(|s| s == "low"),
                    4 => c.adaptive,
                    _ => false,
                };
                valid.then_some(vec![1, *code, 0])
            }
            (Key::Bass, Value::Pair(on, wire))
                if *on <= 1
                    && (c.bass_steps == 3 && [0, 5, 10].contains(wire)
                        || c.bass_steps == 5 && [2, 4, 6, 8, 10].contains(wire)) =>
            {
                Some(vec![*on, *wire])
            }
            (Key::InEar, Value::Flag(v)) => Some(vec![1, 1, u8::from(*v)]),
            (Key::LowLatency, Value::Flag(v)) => Some(vec![if *v { 1 } else { 2 }, 0]),
            (Key::Personal | Key::Dual, Value::Flag(v)) => Some(vec![u8::from(*v)]),
            (Key::Spatial, Value::Pair(mode, head)) => c
                .spatial_modes
                .iter()
                .any(|s| spatial(&s.key) == (*mode, *head))
                .then_some(vec![*mode, *head]),
            (Key::Preset, Value::Byte(id)) => {
                c.eq.presets
                    .iter()
                    .any(|p| p.id == *id)
                    .then_some(vec![*id, 0])
            }
            (Key::CustomEq, Value::Eq(bands)) if bands.iter().all(|g| (-6..=6).contains(g)) => {
                Some(encode_eq(*bands, c.eq.treble_hz))
            }
            (Key::Gesture(side, slot), Value::Byte(code)) => {
                let profile = c.gestures.as_ref()?;
                let slot = profile.slots.iter().find(|s| s.r#type == slot)?;
                let valid = slot.actions.iter().any(|a| {
                    a.value.as_u64() == Some(u64::from(*code))
                        || a.value.as_str() == Some("noise") && [10, 20, 21, 22].contains(code)
                });
                valid.then_some(vec![1, side, 1, slot.r#type, *code])
            }
            _ => None,
        }
    }

    pub fn action(&mut self, action: Action, now: Instant) {
        if !self.connected {
            return;
        }
        match action {
            Action::Set(key, target) => {
                // EQ sliders can emit many targets before the first readback.
                // Keep the latest target and at most one unsent EQ write.
                // A settled value must still be sent if an older write is pending.
                if (self.is_pending(key) && key != Key::CustomEq)
                    || (!self.is_pending(key) && self.value(key) == Some(&target))
                {
                    return;
                }
                let Some((_, set, _)) = self.spec(key) else {
                    return;
                };
                let Some(payload) = self.payload(key, &target) else {
                    return;
                };
                if self.simulation {
                    self.values.insert(key, target);
                    return;
                }
                if key == Key::CustomEq {
                    self.queue.retain(|(command, _)| *command != set);
                }
                self.enqueue(set, payload);
                self.pending.insert(
                    key,
                    Pending {
                        target,
                        next: now + Duration::from_millis(500),
                        deadline: now + Duration::from_secs(5),
                    },
                );
                self.message = "Waiting for the headphones to confirm the change…".into();
            }
            Action::Refresh => {
                if !self.simulation && self.queue.is_empty() {
                    self.query_all();
                }
            }
            Action::Ring(side) => {
                if ![2, 3, 6].contains(&side) || self.caps.is_none() {
                    return;
                }
                self.stop_ring();
                self.ringing = Some(side);
                self.ring_stop = Some(now + Duration::from_secs(30));
                let p = self.ring_payload(side, true);
                self.enqueue(0xf002, p);
            }
            Action::StopRing => self.stop_ring(),
            Action::DualConnect(address, connect) => {
                if self.dual_busy.is_some()
                    || !self.flag(Key::Dual)
                    || !self
                        .devices
                        .iter()
                        .any(|d| d.address == address && !d.is_self && d.connected != connect)
                {
                    return;
                }
                if connect && self.devices.iter().filter(|d| d.connected).count() >= 2 {
                    return;
                }
                if self.simulation {
                    if let Some(d) = self.devices.iter_mut().find(|d| d.address == address) {
                        d.connected = connect;
                    }
                    return;
                }
                let mut p = vec![u8::from(connect)];
                p.extend(address);
                self.enqueue(0xf01b, p);
                self.dual_busy = Some((address, now + Duration::from_secs(15)));
            }
        }
    }
    fn ring_payload(&self, side: u8, on: bool) -> Vec<u8> {
        match self.caps.as_ref().map(|c| c.ring.as_str()) {
            Some("single") => vec![u8::from(on)],
            Some("headphone") => vec![6, u8::from(on)],
            _ => vec![side, u8::from(on)],
        }
    }
    pub fn stop_ring(&mut self) {
        if let Some(side) = self.ringing.take() {
            let p = self.ring_payload(side, false);
            if self.connected && !self.simulation {
                self.queue.push_front((0xf002, p));
            }
        }
        self.ring_stop = None;
    }
    fn report(&mut self, key: Key, value: Value) {
        if let Some(p) = self.pending.get(&key) {
            if p.target != value {
                return;
            } // Stale read while a write is settling.
            self.pending.remove(&key);
            if self.pending.is_empty() {
                self.message = "Settings confirmed by the headphones.".into();
            }
        }
        self.values.insert(key, value);
    }
    pub fn receive(&mut self, chunk: &[u8]) {
        if !self.connected {
            return;
        }
        for frame in self.parser.push(chunk) {
            self.frame(frame);
        }
    }
    fn frame(&mut self, frame: Frame) {
        let p = &frame.payload;
        match frame.command {
            0x4007 | 0xe001 => {
                let Some(&count) = p.first() else {
                    return;
                };
                if p.len() < 1 + usize::from(count) * 2 {
                    return;
                }
                let mut battery = BTreeMap::new();
                for pair in p[1..].as_chunks::<2>().0.iter().take(usize::from(count)) {
                    if [2, 3, 4, 6].contains(&pair[0]) && pair[1] & 0x7f <= 100 {
                        battery.insert(
                            pair[0],
                            Battery {
                                level: pair[1] & 0x7f,
                                charging: pair[1] & 0x80 != 0,
                            },
                        );
                    }
                }
                self.battery = battery;
            }
            0x401e | 0xe003 | 0x6003 if p.len() >= 2 && [1, 2, 3, 4, 5, 7].contains(&p[1]) => {
                self.report(Key::Noise, Value::Byte(p[1]))
            }
            0x404e if p.len() >= 2 && p[0] <= 1 => {
                if p[1] == 5
                    && let Some(c) = &mut self.caps
                    && c.bass_steps == 5
                {
                    c.bass_steps = 3;
                }
                self.report(Key::Bass, Value::Pair(p[0], p[1]));
            }
            0x400e if p.len() >= 3 && p[2] <= 1 => self.report(Key::InEar, Value::Flag(p[2] == 1)),
            0x4041 if p.first().is_some_and(|v| [1, 2].contains(v)) => {
                self.report(Key::LowLatency, Value::Flag(p[0] == 1))
            }
            0x4022 | 0x405a | 0xe019 | 0x6019 if p.first().is_some_and(|v| *v <= 1) => {
                self.report(Key::Personal, Value::Flag(p[0] == 1))
            }
            0x404f if p.len() >= 2 => self.report(Key::Spatial, Value::Pair(p[0], p[1])),
            0x401f | 0x4050 if !p.is_empty() => self.report(Key::Preset, Value::Byte(p[0])),
            0x4044 => {
                if let Some(bands) = decode_eq(p) {
                    self.report(Key::CustomEq, Value::Eq(bands));
                }
            }
            0x4027 if p.first().is_some_and(|v| *v <= 1) => {
                let was = self.flag(Key::Dual);
                self.report(Key::Dual, Value::Flag(p[0] == 1));
                if !was && self.flag(Key::Dual) {
                    self.refresh_dual();
                }
            }
            0x4028 => {
                let Some(page) = decode_dual(p) else {
                    return;
                };
                let mut added = false;
                for device in page {
                    if let Some(d) = self
                        .devices
                        .iter_mut()
                        .find(|d| d.address == device.address)
                    {
                        *d = device;
                    } else {
                        self.devices.push(device);
                        added = true;
                    }
                }
                if added && self.dual_pages < 20 && self.devices.len() < 255 {
                    self.dual_pages += 1;
                    self.enqueue(0xc028, vec![self.devices.len() as u8]);
                }
                self.devices.sort_by(|a, b| {
                    b.is_self
                        .cmp(&a.is_self)
                        .then(b.connected.cmp(&a.connected))
                        .then(a.name.cmp(&b.name))
                });
            }
            0xe00e if p.len() >= 7 => {
                if let Some(d) = self.devices.iter_mut().find(|d| d.address == p[1..7]) {
                    d.connected = p[0] == 1;
                } else {
                    self.refresh_dual();
                }
                if self.dual_busy.is_some_and(|(mac, _)| mac == p[1..7]) {
                    self.dual_busy = None;
                }
            }
            0x4018 if !p.is_empty() => {
                if p.len() < 1 + usize::from(p[0]) * 4 {
                    return;
                }
                for r in p[1..].as_chunks::<4>().0.iter().take(usize::from(p[0])) {
                    self.report(Key::Gesture(r[0], r[2]), Value::Byte(r[3]));
                }
            }
            0x4042 => {
                self.firmware = String::from_utf8_lossy(p)
                    .trim_matches('\0')
                    .trim()
                    .to_string()
            }
            _ => {}
        }
    }

    /// No fixed UI tick: return the next actual protocol deadline.
    pub fn deadline(&self) -> Option<Instant> {
        if !self.connected {
            return None;
        }
        let mut next = if self.simulation {
            None
        } else {
            Some(self.next_battery)
        };
        for at in self
            .pending
            .values()
            .flat_map(|p| [p.next, p.deadline])
            .chain(self.ring_stop)
            .chain(self.dual_busy.map(|(_, at)| at))
            .chain((!self.queue.is_empty()).then_some(self.next_send))
        {
            next = Some(next.map_or(at, |old| old.min(at)));
        }
        next
    }
    pub fn tick(&mut self, now: Instant) -> Vec<Vec<u8>> {
        if !self.connected {
            return vec![];
        }
        if self.ring_stop.is_some_and(|at| now >= at) {
            self.stop_ring();
        }
        if self.dual_busy.is_some_and(|(_, at)| now >= at) {
            self.dual_busy = None;
            self.refresh_dual();
        }
        let due: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, p)| now >= p.next || now >= p.deadline)
            .map(|(&k, _)| k)
            .collect();
        for key in due {
            if self.pending.get(&key).is_some_and(|p| now >= p.deadline) {
                self.pending.remove(&key);
                self.values.remove(&key);
                self.message =
                    "A setting was not confirmed. Reading its current value again.".into();
            } else if let Some(p) = self.pending.get_mut(&key) {
                p.next = now + Duration::from_millis(700);
            }
            self.read(key);
        }
        if !self.simulation && now >= self.next_battery {
            self.enqueue(0xc007, vec![]);
            self.next_battery = now + Duration::from_secs(60);
        }
        if now >= self.next_send
            && let Some((command, payload)) = self.queue.pop_front()
        {
            self.operation = self.operation.wrapping_add(1);
            self.next_send = now + Duration::from_millis(150);
            if let Ok(frame) = protocol::encode(command, &payload, self.operation) {
                return vec![frame];
            }
        }
        vec![]
    }
}

pub fn spatial(key: &str) -> (u8, u8) {
    match key {
        "headTracked" => (1, 1),
        "fixed" => (1, 0),
        "concert" => (2, 0),
        "theatre" => (3, 0),
        "game" => (4, 0),
        _ => (0, 0),
    }
}
pub fn encode_eq(bands: [i8; 3], treble: u16) -> Vec<u8> {
    let mut p = vec![3];
    p.extend((-f32::from(*bands.iter().max().unwrap_or(&0)).max(0.)).to_le_bytes());
    for (kind, gain, freq, q) in [
        (1, bands[1], 980., 0.7),
        (2, bands[2], f32::from(treble), 1.),
        (0, bands[0], 140., 0.8),
    ] {
        p.push(kind);
        p.extend(f32::from(gain).to_le_bytes());
        p.extend(freq.to_le_bytes());
        p.extend(f32::to_le_bytes(q));
    }
    p.extend([0; 9]);
    p
}
fn decode_eq(p: &[u8]) -> Option<[i8; 3]> {
    let count = usize::from(*p.first()?);
    if count != 3 || p.len() < 5 + count * 13 {
        return None;
    }
    let mut bands = [0; 3];
    let mut seen = [false; 3];
    for i in 0..count {
        let at = 5 + i * 13;
        let kind = usize::from(p[at]);
        if kind > 2 || seen[kind] {
            return None;
        }
        let gain = f32::from_le_bytes(p[at + 1..at + 5].try_into().ok()?);
        if !gain.is_finite() || !(-6.0..=6.0).contains(&gain) {
            return None;
        }
        bands[kind] = gain.round() as i8;
        seen[kind] = true;
    }
    Some(bands)
}
fn decode_dual(p: &[u8]) -> Option<Vec<DualDevice>> {
    let count = usize::from(*p.get(2)?);
    let mut at = 3;
    let mut devices = Vec::new();
    for _ in 0..count {
        let header = p.get(at..at + 8)?;
        let length = usize::from(header[7] & 0x7f);
        let name = String::from_utf8_lossy(p.get(at + 8..at + 8 + length)?).into_owned();
        devices.push(DualDevice {
            address: header[1..7].try_into().ok()?,
            name,
            is_self: header[0] & 0xf0 != 0,
            connected: header[0] & 0xf != 0,
        });
        at += 8 + length;
    }
    Some(devices)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn report(d: &mut Device, cmd: u16, p: &[u8]) {
        d.receive(&protocol::encode(cmd, p, 1).unwrap());
    }
    #[test]
    fn battery_masks_and_truncated_reports() {
        let mut d = Device::default();
        d.on_connected("CMF Buds 2 Plus".into(), "test".into());
        report(&mut d, 0x4007, &[2, 2, 87, 3, 0xd0]);
        assert_eq!(
            d.battery[&3],
            Battery {
                level: 80,
                charging: true
            }
        );
        report(&mut d, 0x4007, &[2, 2, 42]);
        assert_eq!(d.battery[&2].level, 87);
        report(&mut d, 0x4007, &[1, 4, 50]);
        assert!(!d.battery.contains_key(&2));
    }
    #[test]
    fn setting_is_only_changed_after_matching_device_confirmation() {
        let mut d = Device::default();
        d.on_connected("Nothing Ear".into(), "test".into());
        let now = Instant::now();
        report(&mut d, 0x401e, &[1, 5, 0]);
        d.action(Action::Set(Key::Noise, Value::Byte(1)), now);
        report(&mut d, 0x401e, &[1, 5, 0]);
        assert!(d.is_pending(Key::Noise));
        assert_eq!(d.value(Key::Noise), Some(&Value::Byte(5)));
        report(&mut d, 0x401e, &[1, 1, 0]);
        assert!(!d.is_pending(Key::Noise));
        assert_eq!(d.value(Key::Noise), Some(&Value::Byte(1)));
    }
    #[test]
    fn disconnect_cancels_every_scheduled_write() {
        let mut d = Device::default();
        d.on_connected("Nothing Ear".into(), "test".into());
        d.action(Action::Set(Key::Noise, Value::Byte(1)), Instant::now());
        d.disconnect();
        assert!(d.tick(Instant::now() + Duration::from_secs(90)).is_empty());
        assert!(d.deadline().is_none());
        assert!(d.values.is_empty());
    }
    #[test]
    fn timeout_removes_unconfirmed_value_and_releases_control() {
        let mut d = Device::default();
        d.on_connected("Nothing Ear".into(), "test".into());
        let now = Instant::now();
        d.action(Action::Set(Key::Noise, Value::Byte(1)), now);
        d.tick(now + Duration::from_secs(6));
        assert!(!d.is_pending(Key::Noise));
        assert!(d.value(Key::Noise).is_none());
    }
    #[test]
    fn unknown_device_cannot_receive_feature_writes() {
        let mut d = Device::default();
        d.on_connected("Renamed headphones".into(), "test".into());
        d.queue.clear();
        d.action(Action::Set(Key::Noise, Value::Byte(1)), Instant::now());
        assert!(d.queue.is_empty());
    }
    #[test]
    fn eq_encoding_matches_filter_layout_and_roundtrips() {
        let payload = encode_eq([3, -2, 6], 6900);
        assert_eq!(payload.len(), 53);
        assert_eq!(
            protocol::hex(&protocol::encode(0xf041, &payload, 3).unwrap()),
            "55600141f0350003030000c0c001000000c0000075443333333f020000c04000a0d7450000803f000000404000000c43cdcc4c3f00000000000000000056a0"
        );
        assert_eq!(f32::from_le_bytes(payload[1..5].try_into().unwrap()), -6.);
        assert_eq!(decode_eq(&payload), Some([3, -2, 6]));
        assert!(decode_eq(&[255]).is_none());
    }
    #[test]
    fn ring_automatically_stops_and_simulation_never_writes() {
        let mut d = Device::simulate("CMF Buds 2 Plus");
        let now = Instant::now();
        d.action(Action::Ring(2), now);
        assert_eq!(d.ringing, Some(2));
        assert!(d.tick(now + Duration::from_secs(31)).is_empty());
        assert!(d.ringing.is_none());
    }
    #[test]
    fn dual_self_and_connection_limit_are_protected() {
        let mut d = Device::simulate("CMF Buds 2 Plus");
        let now = Instant::now();
        d.action(Action::DualConnect([1, 2, 3, 4, 5, 6], false), now);
        assert!(d.devices[0].connected);
        d.action(Action::DualConnect([6, 5, 4, 3, 2, 1], true), now);
        assert!(d.devices[1].connected);
        assert!(decode_dual(&[0, 0, 1, 0, 1]).is_none());
    }
    #[test]
    fn rapid_eq_changes_keep_the_final_value_including_return_to_initial() {
        let mut d = Device::default();
        d.on_connected("CMF Buds 2 Plus".into(), "test".into());
        d.queue.clear();
        report(&mut d, 0x4044, &encode_eq([0, 0, 0], 3400));
        let now = Instant::now();
        d.action(Action::Set(Key::CustomEq, Value::Eq([1, 0, 0])), now);
        assert_eq!(d.tick(now).len(), 1); // First write is already on the wire.
        for bass in [2, 3, 4, 5, 6, 0] {
            d.action(Action::Set(Key::CustomEq, Value::Eq([bass, 0, 0])), now);
        }
        assert_eq!(d.queue.len(), 1);
        report(&mut d, 0x4044, &encode_eq([1, 0, 0], 3400));
        assert!(d.is_pending(Key::CustomEq));
        let frames = Parser::default().push(&d.tick(now + Duration::from_millis(151))[0]);
        assert_eq!(frames[0].command, 0xf041);
        assert_eq!(decode_eq(&frames[0].payload), Some([0, 0, 0]));
        report(&mut d, 0x4044, &encode_eq([0, 0, 0], 3400));
        assert!(!d.is_pending(Key::CustomEq));
    }
    #[test]
    fn fixed_anc_models_accept_anc_but_open_ear_model_does_not() {
        for (name, supported) in [
            ("CMF Buds 2a", true),
            ("CMF Buds", true),
            ("Nothing Ear (open)", false),
        ] {
            let mut d = Device::default();
            d.on_connected(name.into(), "test".into());
            d.queue.clear();
            d.action(Action::Set(Key::Noise, Value::Byte(1)), Instant::now());
            assert_eq!(d.is_pending(Key::Noise), supported, "{name}");
            assert_eq!(
                d.queue
                    .front()
                    .map(|(command, payload)| (*command, payload.as_slice())),
                supported.then_some((0xf00f, [1, 1, 0].as_slice())),
                "{name}"
            );
        }
    }
}
