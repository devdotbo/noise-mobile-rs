package com.example.noisemobile

import android.Manifest
import android.bluetooth.*
import android.bluetooth.le.*
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.ParcelUuid
import android.util.Log
import androidx.annotation.RequiresApi
import androidx.annotation.RequiresPermission
import androidx.core.app.ActivityCompat
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.*
import kotlin.collections.ArrayDeque

/**
 * BLE transport layer for Noise protocol
 */
@RequiresApi(Build.VERSION_CODES.LOLLIPOP)
class BLENoiseTransport(
    private val context: Context,
    private val mode: NoiseMode
) : AutoCloseable {
    
    companion object {
        private const val TAG = "BLENoiseTransport"
        
        // BLE Service and Characteristic UUIDs
        private val SERVICE_UUID = UUID.fromString("12345678-1234-5678-1234-567812345678")
        private val TX_CHAR_UUID = UUID.fromString("12345678-1234-5678-1234-567812345679")
        private val RX_CHAR_UUID = UUID.fromString("12345678-1234-5678-1234-567812345680")
        
        // BLE MTU
        private const val DEFAULT_MTU = 20
        private const val MAX_MTU = 512
    }
    
    // Noise session
    private var noiseSession: NoiseSession? = null
    
    // BLE components
    private val bluetoothManager = context.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager
    private val bluetoothAdapter = bluetoothManager.adapter
    
    // Central mode (client)
    private var bluetoothLeScanner: BluetoothLeScanner? = null
    private var bluetoothGatt: BluetoothGatt? = null
    
    // Peripheral mode (server)
    private var bluetoothGattServer: BluetoothGattServer? = null
    private var bluetoothLeAdvertiser: BluetoothLeAdvertiser? = null
    
    // Characteristics
    private var txCharacteristic: BluetoothGattCharacteristic? = null
    private var rxCharacteristic: BluetoothGattCharacteristic? = null
    
    // Message handling
    private val messageBuffer = ByteBuffer.allocate(65535)
    private val messageQueue = ArrayDeque<ByteArray>()
    private var messageHandler: ((ByteArray) -> Unit)? = null
    
    // Connection state
    var isConnected = false
        private set
    var isHandshakeComplete = false
        private set
    
    private var connectedDevice: BluetoothDevice? = null
    private var currentMtu = DEFAULT_MTU
    
    /**
     * Start as central (client/initiator)
     */
    @RequiresPermission(allOf = [
        Manifest.permission.BLUETOOTH_SCAN,
        Manifest.permission.BLUETOOTH_CONNECT
    ])
    fun startCentral(messageHandler: (ByteArray) -> Unit) {
        this.messageHandler = messageHandler
        noiseSession = NoiseSession.create(NoiseMode.INITIATOR)
        
        bluetoothLeScanner = bluetoothAdapter.bluetoothLeScanner
        startScanning()
    }
    
    /**
     * Start as peripheral (server/responder)
     */
    @RequiresPermission(allOf = [
        Manifest.permission.BLUETOOTH_ADVERTISE,
        Manifest.permission.BLUETOOTH_CONNECT
    ])
    fun startPeripheral(messageHandler: (ByteArray) -> Unit) {
        this.messageHandler = messageHandler
        noiseSession = NoiseSession.create(NoiseMode.RESPONDER)
        
        startGattServer()
        startAdvertising()
    }
    
    /**
     * Send encrypted message
     */
    fun sendMessage(message: ByteArray) {
        val session = noiseSession ?: throw IllegalStateException("No session")
        
        if (!isHandshakeComplete) {
            throw IllegalStateException("Handshake not complete")
        }
        
        val encrypted = session.encrypt(message)
        sendRawData(encrypted)
    }
    
    // Central mode: Scanning
    @RequiresPermission(Manifest.permission.BLUETOOTH_SCAN)
    private fun startScanning() {
        val scanSettings = ScanSettings.Builder()
            .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
            .build()
        
        val scanFilter = ScanFilter.Builder()
            .setServiceUuid(ParcelUuid(SERVICE_UUID))
            .build()
        
        bluetoothLeScanner?.startScan(
            listOf(scanFilter),
            scanSettings,
            scanCallback
        )
        
        Log.d(TAG, "Started scanning for Noise service")
    }
    
    private val scanCallback = object : ScanCallback() {
        @RequiresPermission(Manifest.permission.BLUETOOTH_CONNECT)
        override fun onScanResult(callbackType: Int, result: ScanResult) {
            Log.d(TAG, "Found device: ${result.device.address}")
            
            bluetoothLeScanner?.stopScan(this)
            connectToDevice(result.device)
        }
        
        override fun onScanFailed(errorCode: Int) {
            Log.e(TAG, "Scan failed: $errorCode")
        }
    }
    
    @RequiresPermission(Manifest.permission.BLUETOOTH_CONNECT)
    private fun connectToDevice(device: BluetoothDevice) {
        Log.d(TAG, "Connecting to device: ${device.address}")
        
        bluetoothGatt = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            device.connectGatt(context, false, gattCallback, BluetoothDevice.TRANSPORT_LE)
        } else {
            device.connectGatt(context, false, gattCallback)
        }
    }
    
    private val gattCallback = object : BluetoothGattCallback() {
        @RequiresPermission(Manifest.permission.BLUETOOTH_CONNECT)
        override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
            when (newState) {
                BluetoothProfile.STATE_CONNECTED -> {
                    Log.d(TAG, "Connected to GATT server")
                    isConnected = true
                    connectedDevice = gatt.device
                    
                    // Request higher MTU for better performance
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
                        gatt.requestMtu(MAX_MTU)
                    } else {
                        gatt.discoverServices()
                    }
                }
                BluetoothProfile.STATE_DISCONNECTED -> {
                    Log.d(TAG, "Disconnected from GATT server")
                    isConnected = false
                    isHandshakeComplete = false
                    connectedDevice = null
                    
                    // Restart scanning
                    startScanning()
                }
            }
        }
        
        override fun onMtuChanged(gatt: BluetoothGatt, mtu: Int, status: Int) {
            if (status == BluetoothGatt.GATT_SUCCESS) {
                currentMtu = mtu - 3 // Account for ATT header
                Log.d(TAG, "MTU changed to: $mtu (usable: $currentMtu)")
            }
            gatt.discoverServices()
        }
        
        @RequiresPermission(Manifest.permission.BLUETOOTH_CONNECT)
        override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
            if (status == BluetoothGatt.GATT_SUCCESS) {
                val service = gatt.getService(SERVICE_UUID)
                if (service != null) {
                    txCharacteristic = service.getCharacteristic(TX_CHAR_UUID)
                    rxCharacteristic = service.getCharacteristic(RX_CHAR_UUID)
                    
                    // Enable notifications on RX characteristic
                    rxCharacteristic?.let { char ->
                        gatt.setCharacteristicNotification(char, true)
                        
                        val descriptor = char.getDescriptor(
                            UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")
                        )
                        descriptor?.let {
                            it.value = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
                            gatt.writeDescriptor(it)
                        }
                    }
                    
                    // Start handshake as initiator
                    if (mode == NoiseMode.INITIATOR) {
                        startHandshake()
                    }
                }
            }
        }
        
        override fun onCharacteristicChanged(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic
        ) {
            if (characteristic.uuid == RX_CHAR_UUID) {
                handleReceivedData(characteristic.value)
            }
        }
    }
    
    // Peripheral mode: GATT Server
    @RequiresPermission(Manifest.permission.BLUETOOTH_CONNECT)
    private fun startGattServer() {
        bluetoothGattServer = bluetoothManager.openGattServer(context, gattServerCallback)
        
        // Create service
        val service = BluetoothGattService(SERVICE_UUID, BluetoothGattService.SERVICE_TYPE_PRIMARY)
        
        // TX characteristic (clients write, server reads)
        val txChar = BluetoothGattCharacteristic(
            TX_CHAR_UUID,
            BluetoothGattCharacteristic.PROPERTY_WRITE or
                    BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE,
            BluetoothGattCharacteristic.PERMISSION_WRITE
        )
        
        // RX characteristic (server writes, clients read/notify)
        val rxChar = BluetoothGattCharacteristic(
            RX_CHAR_UUID,
            BluetoothGattCharacteristic.PROPERTY_READ or
                    BluetoothGattCharacteristic.PROPERTY_NOTIFY,
            BluetoothGattCharacteristic.PERMISSION_READ
        )
        
        // Add notification descriptor
        val notifyDescriptor = BluetoothGattDescriptor(
            UUID.fromString("00002902-0000-1000-8000-00805f9b34fb"),
            BluetoothGattDescriptor.PERMISSION_READ or BluetoothGattDescriptor.PERMISSION_WRITE
        )
        rxChar.addDescriptor(notifyDescriptor)
        
        service.addCharacteristic(txChar)
        service.addCharacteristic(rxChar)
        
        bluetoothGattServer?.addService(service)
        
        txCharacteristic = txChar
        rxCharacteristic = rxChar
        
        Log.d(TAG, "GATT server started")
    }
    
    private val gattServerCallback = object : BluetoothGattServerCallback() {
        override fun onConnectionStateChange(device: BluetoothDevice, status: Int, newState: Int) {
            when (newState) {
                BluetoothProfile.STATE_CONNECTED -> {
                    Log.d(TAG, "Device connected: ${device.address}")
                    isConnected = true
                    connectedDevice = device
                }
                BluetoothProfile.STATE_DISCONNECTED -> {
                    Log.d(TAG, "Device disconnected: ${device.address}")
                    isConnected = false
                    isHandshakeComplete = false
                    connectedDevice = null
                }
            }
        }
        
        override fun onCharacteristicWriteRequest(
            device: BluetoothDevice,
            requestId: Int,
            characteristic: BluetoothGattCharacteristic,
            preparedWrite: Boolean,
            responseNeeded: Boolean,
            offset: Int,
            value: ByteArray
        ) {
            if (characteristic.uuid == TX_CHAR_UUID) {
                handleReceivedData(value)
                
                if (responseNeeded) {
                    bluetoothGattServer?.sendResponse(
                        device,
                        requestId,
                        BluetoothGatt.GATT_SUCCESS,
                        0,
                        null
                    )
                }
            }
        }
    }
    
    @RequiresPermission(Manifest.permission.BLUETOOTH_ADVERTISE)
    private fun startAdvertising() {
        bluetoothLeAdvertiser = bluetoothAdapter.bluetoothLeAdvertiser
        
        val settings = AdvertiseSettings.Builder()
            .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_LOW_LATENCY)
            .setConnectable(true)
            .setTimeout(0)
            .setTxPowerLevel(AdvertiseSettings.ADVERTISE_TX_POWER_HIGH)
            .build()
        
        val data = AdvertiseData.Builder()
            .setIncludeDeviceName(false)
            .setIncludeTxPowerLevel(false)
            .addServiceUuid(ParcelUuid(SERVICE_UUID))
            .build()
        
        bluetoothLeAdvertiser?.startAdvertising(settings, data, advertiseCallback)
        
        Log.d(TAG, "Started advertising")
    }
    
    private val advertiseCallback = object : AdvertiseCallback() {
        override fun onStartSuccess(settingsInEffect: AdvertiseSettings) {
            Log.d(TAG, "Advertising started successfully")
        }
        
        override fun onStartFailure(errorCode: Int) {
            Log.e(TAG, "Advertising failed: $errorCode")
        }
    }
    
    // Message handling
    private fun handleReceivedData(data: ByteArray) {
        messageBuffer.put(data)
        
        // Try to process complete messages
        processMessages()
    }
    
    private fun processMessages() {
        messageBuffer.flip()
        
        while (messageBuffer.remaining() >= 4) {
            val startPos = messageBuffer.position()
            val length = messageBuffer.int
            
            if (messageBuffer.remaining() < length) {
                // Not enough data yet, rewind
                messageBuffer.position(startPos)
                break
            }
            
            // Extract message
            val message = ByteArray(length)
            messageBuffer.get(message)
            
            // Process message
            processMessage(message)
        }
        
        messageBuffer.compact()
    }
    
    private fun processMessage(data: ByteArray) {
        val session = noiseSession ?: return
        
        try {
            if (!session.isHandshakeComplete) {
                // Handshake message
                val response = session.readMessage(data)
                
                if (response.isNotEmpty()) {
                    // Got handshake payload
                    Log.d(TAG, "Handshake payload received")
                }
                
                // Check if we need to send next handshake message
                if (!session.isHandshakeComplete) {
                    val nextMessage = session.writeMessage()
                    sendRawData(nextMessage)
                } else {
                    // Handshake complete!
                    isHandshakeComplete = true
                    Log.d(TAG, "Noise handshake complete!")
                    
                    // Notify handler
                    messageHandler?.invoke(byteArrayOf())
                }
            } else {
                // Transport message
                val plaintext = session.decrypt(data)
                messageHandler?.invoke(plaintext)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error processing message", e)
        }
    }
    
    private fun startHandshake() {
        val session = noiseSession ?: return
        
        if (mode == NoiseMode.INITIATOR) {
            try {
                val msg1 = session.writeMessage()
                sendRawData(msg1)
            } catch (e: Exception) {
                Log.e(TAG, "Failed to start handshake", e)
            }
        }
    }
    
    @RequiresPermission(Manifest.permission.BLUETOOTH_CONNECT)
    private fun sendRawData(data: ByteArray) {
        // Add length prefix
        val lengthBuffer = ByteBuffer.allocate(4)
            .order(ByteOrder.BIG_ENDIAN)
            .putInt(data.size)
            .array()
        
        val fullMessage = lengthBuffer + data
        
        // Send in chunks
        sendDataInChunks(fullMessage)
    }
    
    @RequiresPermission(Manifest.permission.BLUETOOTH_CONNECT)
    private fun sendDataInChunks(data: ByteArray) {
        var offset = 0
        
        while (offset < data.size) {
            val chunkSize = minOf(currentMtu, data.size - offset)
            val chunk = data.sliceArray(offset until offset + chunkSize)
            
            if (mode == NoiseMode.INITIATOR) {
                // Central writes to peripheral
                txCharacteristic?.let { char ->
                    char.value = chunk
                    bluetoothGatt?.writeCharacteristic(char)
                }
            } else {
                // Peripheral notifies central
                rxCharacteristic?.let { char ->
                    char.value = chunk
                    connectedDevice?.let { device ->
                        bluetoothGattServer?.notifyCharacteristicChanged(device, char, false)
                    }
                }
            }
            
            offset += chunkSize
            
            // Small delay between chunks to avoid overwhelming BLE stack
            if (offset < data.size) {
                Thread.sleep(20)
            }
        }
    }
    
    override fun close() {
        bluetoothLeScanner?.stopScan(scanCallback)
        bluetoothGatt?.close()
        bluetoothLeAdvertiser?.stopAdvertising(advertiseCallback)
        bluetoothGattServer?.close()
        noiseSession?.close()
    }
}