import SwiftUI

@main
struct NoiseMobileExampleApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}

struct ContentView: View {
    @StateObject private var viewModel = NoiseViewModel()
    
    var body: some View {
        NavigationView {
            VStack(spacing: 20) {
                // Mode selection
                Picker("Mode", selection: $viewModel.selectedMode) {
                    Text("Initiator (Client)").tag(NoiseMode.initiator)
                    Text("Responder (Server)").tag(NoiseMode.responder)
                }
                .pickerStyle(SegmentedPickerStyle())
                .padding()
                
                // Connection status
                HStack {
                    Circle()
                        .fill(viewModel.isConnected ? Color.green : Color.red)
                        .frame(width: 10, height: 10)
                    Text(viewModel.connectionStatus)
                }
                
                // Start button
                Button(action: viewModel.toggleConnection) {
                    Text(viewModel.isRunning ? "Stop" : "Start")
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(viewModel.isRunning ? Color.red : Color.blue)
                        .foregroundColor(.white)
                        .cornerRadius(8)
                }
                .padding(.horizontal)
                
                // Message input
                if viewModel.isHandshakeComplete {
                    VStack(alignment: .leading) {
                        Text("Send Message:")
                            .font(.headline)
                        
                        HStack {
                            TextField("Enter message", text: $viewModel.messageText)
                                .textFieldStyle(RoundedBorderTextFieldStyle())
                            
                            Button("Send") {
                                viewModel.sendMessage()
                            }
                            .disabled(viewModel.messageText.isEmpty)
                        }
                    }
                    .padding()
                }
                
                // Message log
                VStack(alignment: .leading) {
                    Text("Messages:")
                        .font(.headline)
                    
                    ScrollView {
                        VStack(alignment: .leading, spacing: 5) {
                            ForEach(viewModel.messages) { message in
                                MessageRow(message: message)
                            }
                        }
                    }
                    .frame(maxHeight: 300)
                    .border(Color.gray.opacity(0.3))
                }
                .padding()
                
                Spacer()
            }
            .navigationTitle("Noise Protocol Demo")
            .alert("Error", isPresented: $viewModel.showError) {
                Button("OK") {
                    viewModel.errorMessage = nil
                }
            } message: {
                Text(viewModel.errorMessage ?? "Unknown error")
            }
        }
    }
}

struct MessageRow: View {
    let message: Message
    
    var body: some View {
        HStack {
            Text(message.timestamp, style: .time)
                .font(.caption)
                .foregroundColor(.gray)
            
            if message.isOutgoing {
                Spacer()
                Text(message.content)
                    .padding(8)
                    .background(Color.blue.opacity(0.2))
                    .cornerRadius(8)
            } else {
                Text(message.content)
                    .padding(8)
                    .background(Color.gray.opacity(0.2))
                    .cornerRadius(8)
                Spacer()
            }
        }
        .padding(.horizontal)
    }
}

// MARK: - View Model

class NoiseViewModel: ObservableObject {
    @Published var selectedMode = NoiseMode.initiator
    @Published var isRunning = false
    @Published var isConnected = false
    @Published var isHandshakeComplete = false
    @Published var connectionStatus = "Disconnected"
    @Published var messages: [Message] = []
    @Published var messageText = ""
    @Published var showError = false
    @Published var errorMessage: String?
    
    private var transport: BLENoiseTransport?
    
    func toggleConnection() {
        if isRunning {
            stop()
        } else {
            start()
        }
    }
    
    private func start() {
        do {
            transport = BLENoiseTransport(mode: selectedMode)
            
            if selectedMode == .initiator {
                try transport?.startCentral { [weak self] data in
                    self?.handleReceivedMessage(data)
                }
                connectionStatus = "Scanning..."
            } else {
                try transport?.startPeripheral { [weak self] data in
                    self?.handleReceivedMessage(data)
                }
                connectionStatus = "Advertising..."
            }
            
            isRunning = true
            
            // Observe connection state
            observeConnectionState()
            
        } catch {
            showError(error)
        }
    }
    
    private func stop() {
        transport = nil
        isRunning = false
        isConnected = false
        isHandshakeComplete = false
        connectionStatus = "Disconnected"
    }
    
    func sendMessage() {
        guard !messageText.isEmpty,
              let transport = transport,
              transport.isHandshakeComplete else { return }
        
        do {
            let data = messageText.data(using: .utf8) ?? Data()
            try transport.sendMessage(data)
            
            // Add to message log
            let message = Message(
                content: messageText,
                isOutgoing: true,
                timestamp: Date()
            )
            messages.append(message)
            messageText = ""
            
        } catch {
            showError(error)
        }
    }
    
    private func handleReceivedMessage(_ data: Data) {
        DispatchQueue.main.async { [weak self] in
            if data.isEmpty {
                // Handshake complete signal
                self?.isHandshakeComplete = true
                self?.connectionStatus = "Secure connection established"
                self?.messages.append(Message(
                    content: "🔒 Noise handshake complete!",
                    isOutgoing: false,
                    timestamp: Date()
                ))
            } else {
                // Regular message
                let text = String(data: data, encoding: .utf8) ?? "Invalid message"
                let message = Message(
                    content: text,
                    isOutgoing: false,
                    timestamp: Date()
                )
                self?.messages.append(message)
            }
        }
    }
    
    private func observeConnectionState() {
        // In a real app, you'd observe the transport's connection state
        Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] timer in
            guard let transport = self?.transport else {
                timer.invalidate()
                return
            }
            
            DispatchQueue.main.async {
                self?.isConnected = transport.isConnected
                self?.isHandshakeComplete = transport.isHandshakeComplete
                
                if transport.isConnected && !transport.isHandshakeComplete {
                    self?.connectionStatus = "Performing handshake..."
                } else if transport.isHandshakeComplete {
                    self?.connectionStatus = "Secure connection established"
                }
            }
        }
    }
    
    private func showError(_ error: Error) {
        errorMessage = error.localizedDescription
        showError = true
    }
}

// MARK: - Models

struct Message: Identifiable {
    let id = UUID()
    let content: String
    let isOutgoing: Bool
    let timestamp: Date
}