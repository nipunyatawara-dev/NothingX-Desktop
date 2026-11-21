// IOBluetooth transport, adapted from Nothing X Mac by Yash Raut.
// Modified 2026-11-17: Swift 6 isolation, framed stdin, bounded buffers,
// delayed discovery, no audio-link resets, transport-only responsibilities.
import Foundation
import IOBluetooth
import CoreBluetooth

@MainActor
func emit(_ object: [String: String]) {
    guard var data = try? JSONSerialization.data(withJSONObject: object) else { return }
    data.append(10)
    FileHandle.standardOutput.write(data)
}

@MainActor
final class BluetoothBridge: NSObject, @preconcurrency IOBluetoothRFCOMMChannelDelegate, @preconcurrency CBCentralManagerDelegate {
    private static let hexDigits: [UInt8] = Array("0123456789abcdef".utf8)
    private let service = IOBluetoothSDPUUID(bytes: [UInt8]([
        0xae, 0xac, 0x4a, 0x03, 0xdf, 0xf5, 0x49, 0x8f,
        0x84, 0x3a, 0x34, 0x48, 0x7c, 0xf1, 0x33, 0xeb,
    ]), length: 16)
    private var channel: IOBluetoothRFCOMMChannel?
    private var device: IOBluetoothDevice?
    private var ready = false
    private var openCompleted = false
    private var openingTimeout: Timer?
    private var input = Data()
    private var retry: Timer?
    private var delay: TimeInterval = 1
    private var stopping = false
    private var sdpRequested = Set<String>()
    private var refreshedAfterFailure = Set<String>()
    private var failedScanCount = 0
    private var lastUnrecognized = ""
    private var permissionManager: CBCentralManager?

    func start() {
        if CBManager.authorization == .notDetermined {
            permissionManager = CBCentralManager(delegate: self, queue: .main)
        } else { scan() }
    }

    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        if central.state == .unauthorized {
            emit(["type": "log", "message": "Bluetooth permission is required in System Settings → Privacy & Security → Bluetooth."])
        } else if central.state == .poweredOn { scan() }
    }

    private func scheduleScan() {
        guard !stopping else { return }
        retry?.invalidate()
        retry = Timer.scheduledTimer(withTimeInterval: delay, repeats: false) { [weak self] _ in
            Task { @MainActor in self?.scan() }
        }
        delay = min(delay * 1.5, 8)
    }

    private func isNothingLike(_ name: String) -> Bool {
        let lower = name.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return lower.contains("nothing")
            || lower.contains("cmf")
            || lower.hasPrefix("ear (")
            || lower.hasPrefix("ear(")
            || lower.contains("buds pro")
            || lower.contains("neckband pro")
    }

    private func reportUnrecognized(_ name: String) {
        guard name != lastUnrecognized else { return }
        lastUnrecognized = name
        emit(["type": "unrecognized", "name": name])
    }

    private func candidatePriority(_ device: IOBluetoothDevice) -> Int {
        if device.getServiceRecord(for: service) != nil { return 0 }
        if isNothingLike(device.name ?? "") { return 1 }
        return 2
    }

    private func scan() {
        guard !stopping, channel == nil else { return }
        let paired = IOBluetoothDevice.pairedDevices() as? [IOBluetoothDevice] ?? []
        let connected = paired.filter { $0.isConnected() }.sorted {
            candidatePriority($0) < candidatePriority($1)
        }
        if ProcessInfo.processInfo.environment["NXM_DEBUG"] == "1" {
            emit(["type": "log", "message": "Bluetooth authorization \(CBManager.authorization.rawValue); paired \(paired.count); connected \(connected.count)"])
        }
        for device in connected {
            if ProcessInfo.processInfo.environment["NXM_DEBUG"] == "1" {
                emit(["type": "log", "message": "Connected Bluetooth candidate: \(device.name ?? "unnamed"); cached services: \(device.services?.count ?? 0)"])
            }
            guard let record = device.getServiceRecord(for: service) else {
                let address = device.addressString ?? ""
                if sdpRequested.insert(address).inserted { device.performSDPQuery(nil) }
                continue
            }
            var id: BluetoothRFCOMMChannelID = 0
            guard record.getRFCOMMChannelID(&id) == kIOReturnSuccess else { continue }
            var opened: IOBluetoothRFCOMMChannel?
            self.device = device
            ready = false
            openCompleted = false
            // Async open keeps the run loop responsive to quit/EOF while the
            // remote device is releasing an earlier control channel.
            let status = device.openRFCOMMChannelAsync(&opened, withChannelID: id, delegate: self)
            guard status == kIOReturnSuccess, let opened else {
                let code = String(format: "0x%08x", UInt32(bitPattern: status))
                emit(["type": "log", "message": "Control channel could not open (\(code), channel \(id)). Close other Nothing controllers; retrying."])
                if refreshedAfterFailure.insert(device.addressString ?? "").inserted { device.performSDPQuery(nil) }
                self.device = nil
                continue
            }
            // A cached channel can complete inside the open call itself.
            if openCompleted { return }
            channel = opened
            openingTimeout = Timer.scheduledTimer(withTimeInterval: 6, repeats: false) { [weak self] _ in
                Task { @MainActor in
                    guard let self, !self.ready, !self.stopping else { return }
                    self.channel?.close()
                    self.channel = nil
                    self.device = nil
                    self.scheduleScan()
                }
            }
            return
        }
        if let candidate = connected.first(where: {
            isNothingLike($0.name ?? "") || $0.getServiceRecord(for: service) != nil
        }) {
            failedScanCount += 1
            if failedScanCount >= 2 {
                reportUnrecognized(candidate.name ?? "Nothing/CMF device")
            }
        } else {
            failedScanCount = 0
            reportUnrecognized("")
        }
        scheduleScan()
    }

    func receive(_ data: Data) {
        // Pipe chunks are not message boundaries. Decode only complete JSON lines.
        input.append(data)
        guard input.count <= 16_384 else { input.removeAll(); return }
        while let end = input.firstIndex(of: 10) {
            let line = input[..<end]
            input.removeSubrange(...end)
            guard let command = try? JSONDecoder().decode(Command.self, from: Data(line)) else { continue }
            if command.type == "quit" { shutdown(); return }
            if command.type == "refresh", channel == nil {
                delay = 1
                sdpRequested.removeAll()
                refreshedAfterFailure.removeAll()
                retry?.invalidate()
                scan()
            }
            if command.type == "write", let hex = command.hex { write(hex) }
        }
    }

    private func write(_ hex: String) {
        guard ready, let channel, hex.utf8.count.isMultiple(of: 2), hex.utf8.count <= 530 else { return }
        let chars = Array(hex.utf8)
        var bytes: [UInt8] = []
        for at in stride(from: 0, to: chars.count, by: 2) {
            guard let byte = UInt8(String(decoding: chars[at...at+1], as: UTF8.self), radix: 16) else { return }
            bytes.append(byte)
        }
        guard !bytes.isEmpty else { return }
        let result = bytes.withUnsafeMutableBytes { buffer in
            channel.writeSync(buffer.baseAddress, length: UInt16(buffer.count))
        }
        if result != kIOReturnSuccess {
            channel.setDelegate(nil)
            channel.close()
            self.channel = nil
            emit(["type": "disconnected"])
            scheduleScan()
        }
    }

    func shutdown() {
        guard !stopping else { return }
        stopping = true
        retry?.invalidate()
        openingTimeout?.invalidate()
        if let channel {
            // Keep the delegate and run loop alive for the close acknowledgement.
            // Exiting immediately can leave a stale RFCOMM session on the buds.
            channel.close()
            DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) { exit(0) }
        } else { exit(0) }
    }

    func rfcommChannelOpenComplete(_ opened: IOBluetoothRFCOMMChannel!, status: IOReturn) {
        guard channel == nil || opened === channel else { return }
        channel = opened
        openCompleted = true
        openingTimeout?.invalidate()
        if stopping { opened.close(); return }
        guard status == kIOReturnSuccess, let device else {
            opened.setDelegate(nil)
            opened.close()
            channel = nil
            self.device = nil
            ready = false
            scheduleScan()
            return
        }
        ready = true
        delay = 1
        failedScanCount = 0
        lastUnrecognized = ""
        emit(["type": "connected", "name": device.name ?? "Nothing device", "address": device.addressString ?? ""])
    }

    func rfcommChannelData(_ channel: IOBluetoothRFCOMMChannel!, data pointer: UnsafeMutableRawPointer!, length: Int) {
        guard channel === self.channel, ready, !stopping else { return }
        guard length > 0, length <= 4096, let pointer else { return }
        let data = Data(bytes: pointer, count: length)
        var encoded = [UInt8]()
        encoded.reserveCapacity(length * 2)
        for byte in data {
            encoded.append(Self.hexDigits[Int(byte >> 4)])
            encoded.append(Self.hexDigits[Int(byte & 0x0f)])
        }
        emit(["type": "data", "hex": String(decoding: encoded, as: UTF8.self)])
    }
    func rfcommChannelClosed(_ closed: IOBluetoothRFCOMMChannel!) {
        guard closed === channel else { return }
        channel = nil
        device = nil
        ready = false
        openingTimeout?.invalidate()
        if stopping {
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) { exit(0) }
            return
        }
        sdpRequested.removeAll()
        emit(["type": "disconnected"])
        scheduleScan()
    }
}

struct Command: Decodable { let type: String; let hex: String? }

let bridge = BluetoothBridge()
var signals: [DispatchSourceSignal] = []
for sig in [SIGTERM, SIGINT, SIGHUP] {
    signal(sig, SIG_IGN)
    let source = DispatchSource.makeSignalSource(signal: sig, queue: .main)
    source.setEventHandler { bridge.shutdown() }
    source.resume()
    signals.append(source)
}
FileHandle.standardInput.readabilityHandler = { handle in
    let data = handle.availableData
    if data.isEmpty {
        handle.readabilityHandler = nil
        Task { @MainActor in bridge.shutdown() }
    } else {
        Task { @MainActor in bridge.receive(data) }
    }
}
bridge.start()
RunLoop.main.run()
