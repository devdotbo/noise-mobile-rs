package com.example.noisemobile

import android.Manifest
import android.bluetooth.BluetoothAdapter
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.util.Log
import android.widget.*
import androidx.activity.result.contract.ActivityResultContracts
import androidx.annotation.RequiresApi
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.text.SimpleDateFormat
import java.util.*

@RequiresApi(Build.VERSION_CODES.LOLLIPOP)
class MainActivity : AppCompatActivity() {
    
    companion object {
        private const val TAG = "NoiseMobileExample"
        private const val REQUEST_PERMISSIONS = 100
    }
    
    // UI components
    private lateinit var modeRadioGroup: RadioGroup
    private lateinit var statusText: TextView
    private lateinit var startButton: Button
    private lateinit var messageInput: EditText
    private lateinit var sendButton: Button
    private lateinit var messageList: ListView
    private lateinit var messageAdapter: ArrayAdapter<String>
    
    // Noise transport
    private var transport: BLENoiseTransport? = null
    private var isRunning = false
    
    // Messages
    private val messages = mutableListOf<String>()
    private val dateFormat = SimpleDateFormat("HH:mm:ss", Locale.getDefault())
    
    // Bluetooth enable launcher
    private val enableBluetoothLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == RESULT_OK) {
            checkPermissionsAndStart()
        } else {
            showMessage("Bluetooth is required for this app")
        }
    }
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        
        initializeViews()
        setupListeners()
        
        // Check if Bluetooth is supported
        val bluetoothAdapter = BluetoothAdapter.getDefaultAdapter()
        if (bluetoothAdapter == null) {
            showMessage("Bluetooth not supported on this device")
            finish()
            return
        }
        
        // Request to enable Bluetooth if not enabled
        if (!bluetoothAdapter.isEnabled) {
            val enableBtIntent = Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE)
            enableBluetoothLauncher.launch(enableBtIntent)
        }
    }
    
    private fun initializeViews() {
        modeRadioGroup = findViewById(R.id.mode_radio_group)
        statusText = findViewById(R.id.status_text)
        startButton = findViewById(R.id.start_button)
        messageInput = findViewById(R.id.message_input)
        sendButton = findViewById(R.id.send_button)
        messageList = findViewById(R.id.message_list)
        
        messageAdapter = ArrayAdapter(this, android.R.layout.simple_list_item_1, messages)
        messageList.adapter = messageAdapter
        
        // Initially disable message input
        messageInput.isEnabled = false
        sendButton.isEnabled = false
    }
    
    private fun setupListeners() {
        startButton.setOnClickListener {
            if (isRunning) {
                stop()
            } else {
                checkPermissionsAndStart()
            }
        }
        
        sendButton.setOnClickListener {
            sendMessage()
        }
    }
    
    private fun checkPermissionsAndStart() {
        val permissions = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            arrayOf(
                Manifest.permission.BLUETOOTH_SCAN,
                Manifest.permission.BLUETOOTH_CONNECT,
                Manifest.permission.BLUETOOTH_ADVERTISE,
                Manifest.permission.ACCESS_FINE_LOCATION
            )
        } else {
            arrayOf(
                Manifest.permission.BLUETOOTH,
                Manifest.permission.BLUETOOTH_ADMIN,
                Manifest.permission.ACCESS_FINE_LOCATION
            )
        }
        
        val missingPermissions = permissions.filter {
            ContextCompat.checkSelfPermission(this, it) != PackageManager.PERMISSION_GRANTED
        }
        
        if (missingPermissions.isNotEmpty()) {
            ActivityCompat.requestPermissions(
                this,
                missingPermissions.toTypedArray(),
                REQUEST_PERMISSIONS
            )
        } else {
            start()
        }
    }
    
    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        
        if (requestCode == REQUEST_PERMISSIONS) {
            if (grantResults.all { it == PackageManager.PERMISSION_GRANTED }) {
                start()
            } else {
                showMessage("Permissions required for BLE operation")
            }
        }
    }
    
    private fun start() {
        val mode = when (modeRadioGroup.checkedRadioButtonId) {
            R.id.radio_initiator -> NoiseMode.INITIATOR
            R.id.radio_responder -> NoiseMode.RESPONDER
            else -> return
        }
        
        try {
            transport = BLENoiseTransport(this, mode)
            
            if (mode == NoiseMode.INITIATOR) {
                transport?.startCentral { data ->
                    handleReceivedMessage(data)
                }
                updateStatus("Scanning for devices...")
            } else {
                transport?.startPeripheral { data ->
                    handleReceivedMessage(data)
                }
                updateStatus("Advertising...")
            }
            
            isRunning = true
            startButton.text = "Stop"
            modeRadioGroup.isEnabled = false
            
            // Start monitoring connection state
            monitorConnectionState()
            
        } catch (e: Exception) {
            Log.e(TAG, "Failed to start", e)
            showMessage("Failed to start: ${e.message}")
        }
    }
    
    private fun stop() {
        transport?.close()
        transport = null
        
        isRunning = false
        startButton.text = "Start"
        modeRadioGroup.isEnabled = true
        messageInput.isEnabled = false
        sendButton.isEnabled = false
        
        updateStatus("Stopped")
    }
    
    private fun sendMessage() {
        val text = messageInput.text.toString()
        if (text.isEmpty()) return
        
        lifecycleScope.launch {
            try {
                withContext(Dispatchers.IO) {
                    transport?.sendMessage(text.toByteArray())
                }
                
                // Add to message list
                addMessage("You: $text", true)
                messageInput.text.clear()
                
            } catch (e: Exception) {
                Log.e(TAG, "Failed to send message", e)
                showMessage("Failed to send: ${e.message}")
            }
        }
    }
    
    private fun handleReceivedMessage(data: ByteArray) {
        runOnUiThread {
            if (data.isEmpty()) {
                // Handshake complete signal
                updateStatus("Secure connection established")
                messageInput.isEnabled = true
                sendButton.isEnabled = true
                addMessage("🔒 Noise handshake complete!", false)
            } else {
                // Regular message
                val text = String(data)
                addMessage("Peer: $text", false)
            }
        }
    }
    
    private fun monitorConnectionState() {
        lifecycleScope.launch {
            while (isRunning) {
                val t = transport
                if (t != null) {
                    val status = when {
                        t.isHandshakeComplete -> "Connected (Secure)"
                        t.isConnected -> "Performing handshake..."
                        else -> {
                            val mode = when (modeRadioGroup.checkedRadioButtonId) {
                                R.id.radio_initiator -> "Scanning..."
                                R.id.radio_responder -> "Advertising..."
                                else -> "Unknown"
                            }
                            mode
                        }
                    }
                    updateStatus(status)
                }
                
                kotlinx.coroutines.delay(500)
            }
        }
    }
    
    private fun addMessage(message: String, isOutgoing: Boolean) {
        val timestamp = dateFormat.format(Date())
        val formattedMessage = "[$timestamp] $message"
        
        messages.add(formattedMessage)
        messageAdapter.notifyDataSetChanged()
        
        // Scroll to bottom
        messageList.setSelection(messages.size - 1)
    }
    
    private fun updateStatus(status: String) {
        runOnUiThread {
            statusText.text = "Status: $status"
        }
    }
    
    private fun showMessage(message: String) {
        runOnUiThread {
            Toast.makeText(this, message, Toast.LENGTH_SHORT).show()
        }
    }
    
    override fun onDestroy() {
        super.onDestroy()
        stop()
    }
}