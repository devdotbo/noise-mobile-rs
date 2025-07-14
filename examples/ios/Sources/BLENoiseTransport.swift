import Foundation
import CoreBluetooth

/// BLE transport layer for Noise protocol
public class BLENoiseTransport: NSObject {
    // BLE Service and Characteristic UUIDs
    static let serviceUUID = CBUUID(string: "12345678-1234-5678-1234-567812345678")
    static let txCharacteristicUUID = CBUUID(string: "12345678-1234-5678-1234-567812345679")
    static let rxCharacteristicUUID = CBUUID(string: "12345678-1234-5678-1234-567812345680")
    
    // Noise session
    private var noiseSession: NoiseSession?
    private let mode: NoiseMode
    
    // BLE properties
    private var centralManager: CBCentralManager?
    private var peripheralManager: CBPeripheralManager?
    private var discoveredPeripheral: CBPeripheral?
    private var txCharacteristic: CBCharacteristic?
    private var rxCharacteristic: CBCharacteristic?
    
    // Message handling
    private var messageBuffer = Data()
    private var messageHandler: ((Data) -> Void)?
    
    // Connection state
    public private(set) var isConnected = false
    public private(set) var isHandshakeComplete = false
    
    public init(mode: NoiseMode) {
        self.mode = mode
        super.init()
    }
    
    /// Start as central (client/initiator)
    public func startCentral(messageHandler: @escaping (Data) -> Void) throws {
        self.messageHandler = messageHandler
        self.noiseSession = try NoiseSession(mode: .initiator)
        
        centralManager = CBCentralManager(delegate: self, queue: nil)
    }
    
    /// Start as peripheral (server/responder)
    public func startPeripheral(messageHandler: @escaping (Data) -> Void) throws {
        self.messageHandler = messageHandler
        self.noiseSession = try NoiseSession(mode: .responder)
        
        peripheralManager = CBPeripheralManager(delegate: self, queue: nil)
    }
    
    /// Send encrypted message
    public func sendMessage(_ message: Data) throws {
        guard let session = noiseSession else {
            throw NoiseError.invalidState
        }
        
        let encrypted = try session.encrypt(message)
        sendRawData(encrypted)
    }
    
    /// Handle received data
    private func handleReceivedData(_ data: Data) {
        messageBuffer.append(data)
        
        // Try to process complete messages (simple length-prefixed protocol)
        while messageBuffer.count >= 4 {
            let lengthData = messageBuffer.prefix(4)
            let length = lengthData.withUnsafeBytes { $0.load(as: UInt32.self) }
            
            guard messageBuffer.count >= 4 + Int(length) else { break }
            
            // Extract message
            let messageData = messageBuffer.subdata(in: 4..<(4 + Int(length)))
            messageBuffer.removeFirst(4 + Int(length))
            
            // Process message
            processMessage(messageData)
        }
    }
    
    /// Process a complete message
    private func processMessage(_ data: Data) {
        guard let session = noiseSession else { return }
        
        do {
            if !session.isHandshakeComplete {
                // Handshake message
                let response = try session.readMessage(data)
                
                if !response.isEmpty {
                    // Got handshake payload
                    print("Handshake payload: \(response)")
                }
                
                // Check if we need to send next handshake message
                if !session.isHandshakeComplete {
                    let nextMessage = try session.writeMessage()
                    sendRawData(nextMessage)
                } else {
                    // Handshake complete!
                    isHandshakeComplete = true
                    print("Noise handshake complete!")
                    
                    // If initiator, can start sending application messages
                    if mode == .initiator {
                        messageHandler?(Data())
                    }
                }
            } else {
                // Transport message
                let plaintext = try session.decrypt(data)
                messageHandler?(plaintext)
            }
        } catch {
            print("Error processing message: \(error)")
        }
    }
    
    /// Send raw data with length prefix
    private func sendRawData(_ data: Data) {
        var lengthData = Data(count: 4)
        lengthData.withUnsafeMutableBytes { bytes in
            bytes.storeBytes(of: UInt32(data.count), as: UInt32.self)
        }
        
        let fullMessage = lengthData + data
        
        // Send via BLE (chunked if necessary)
        if let peripheral = discoveredPeripheral, let tx = txCharacteristic {
            // Central sending to peripheral
            sendDataInChunks(fullMessage, to: peripheral, characteristic: tx)
        } else if let manager = peripheralManager {
            // Peripheral sending to central
            // This would be implemented via notifications
            print("Peripheral would send: \(fullMessage.count) bytes")
        }
    }
    
    /// Send data in BLE-sized chunks
    private func sendDataInChunks(_ data: Data, to peripheral: CBPeripheral, characteristic: CBCharacteristic) {
        let mtu = peripheral.maximumWriteValueLength(for: .withoutResponse)
        var offset = 0
        
        while offset < data.count {
            let chunkSize = min(mtu, data.count - offset)
            let chunk = data.subdata(in: offset..<(offset + chunkSize))
            
            peripheral.writeValue(chunk, for: characteristic, type: .withoutResponse)
            offset += chunkSize
        }
    }
}

// MARK: - CBCentralManagerDelegate

extension BLENoiseTransport: CBCentralManagerDelegate {
    public func centralManagerDidUpdateState(_ central: CBCentralManager) {
        switch central.state {
        case .poweredOn:
            print("Central: Bluetooth powered on, scanning...")
            central.scanForPeripherals(withServices: [BLENoiseTransport.serviceUUID], options: nil)
        case .poweredOff:
            print("Central: Bluetooth powered off")
        default:
            print("Central: Bluetooth state: \(central.state.rawValue)")
        }
    }
    
    public func centralManager(_ central: CBCentralManager, didDiscover peripheral: CBPeripheral, advertisementData: [String : Any], rssi RSSI: NSNumber) {
        print("Central: Discovered peripheral: \(peripheral.name ?? "Unknown")")
        
        discoveredPeripheral = peripheral
        peripheral.delegate = self
        central.stopScan()
        central.connect(peripheral, options: nil)
    }
    
    public func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
        print("Central: Connected to peripheral")
        isConnected = true
        peripheral.discoverServices([BLENoiseTransport.serviceUUID])
    }
    
    public func centralManager(_ central: CBCentralManager, didDisconnectPeripheral peripheral: CBPeripheral, error: Error?) {
        print("Central: Disconnected from peripheral")
        isConnected = false
        isHandshakeComplete = false
        discoveredPeripheral = nil
        
        // Restart scanning
        central.scanForPeripherals(withServices: [BLENoiseTransport.serviceUUID], options: nil)
    }
}

// MARK: - CBPeripheralDelegate

extension BLENoiseTransport: CBPeripheralDelegate {
    public func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
        guard let services = peripheral.services else { return }
        
        for service in services {
            if service.uuid == BLENoiseTransport.serviceUUID {
                peripheral.discoverCharacteristics([BLENoiseTransport.txCharacteristicUUID, BLENoiseTransport.rxCharacteristicUUID], for: service)
            }
        }
    }
    
    public func peripheral(_ peripheral: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?) {
        guard let characteristics = service.characteristics else { return }
        
        for characteristic in characteristics {
            if characteristic.uuid == BLENoiseTransport.txCharacteristicUUID {
                txCharacteristic = characteristic
            } else if characteristic.uuid == BLENoiseTransport.rxCharacteristicUUID {
                rxCharacteristic = characteristic
                peripheral.setNotifyValue(true, for: characteristic)
            }
        }
        
        // Start handshake if we have both characteristics
        if txCharacteristic != nil && rxCharacteristic != nil {
            startHandshake()
        }
    }
    
    public func peripheral(_ peripheral: CBPeripheral, didUpdateValueFor characteristic: CBCharacteristic, error: Error?) {
        guard characteristic.uuid == BLENoiseTransport.rxCharacteristicUUID,
              let data = characteristic.value else { return }
        
        handleReceivedData(data)
    }
    
    private func startHandshake() {
        guard let session = noiseSession, mode == .initiator else { return }
        
        do {
            // Send first handshake message
            let msg1 = try session.writeMessage()
            sendRawData(msg1)
        } catch {
            print("Failed to start handshake: \(error)")
        }
    }
}

// MARK: - CBPeripheralManagerDelegate

extension BLENoiseTransport: CBPeripheralManagerDelegate {
    public func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
        switch peripheral.state {
        case .poweredOn:
            print("Peripheral: Bluetooth powered on, setting up service...")
            setupPeripheralService()
        case .poweredOff:
            print("Peripheral: Bluetooth powered off")
        default:
            print("Peripheral: Bluetooth state: \(peripheral.state.rawValue)")
        }
    }
    
    private func setupPeripheralService() {
        guard let manager = peripheralManager else { return }
        
        // Create characteristics
        let txChar = CBMutableCharacteristic(
            type: BLENoiseTransport.txCharacteristicUUID,
            properties: [.write, .writeWithoutResponse],
            value: nil,
            permissions: [.writeable]
        )
        
        let rxChar = CBMutableCharacteristic(
            type: BLENoiseTransport.rxCharacteristicUUID,
            properties: [.notify, .read],
            value: nil,
            permissions: [.readable]
        )
        
        // Create service
        let service = CBMutableService(type: BLENoiseTransport.serviceUUID, primary: true)
        service.characteristics = [txChar, rxChar]
        
        // Add service and start advertising
        manager.add(service)
        manager.startAdvertising([
            CBAdvertisementDataServiceUUIDsKey: [BLENoiseTransport.serviceUUID]
        ])
        
        print("Peripheral: Advertising started")
    }
    
    public func peripheralManager(_ peripheral: CBPeripheralManager, didReceiveWrite requests: [CBATTRequest]) {
        for request in requests {
            if request.characteristic.uuid == BLENoiseTransport.txCharacteristicUUID {
                if let data = request.value {
                    handleReceivedData(data)
                }
                peripheral.respond(to: request, withResult: .success)
            }
        }
    }
}