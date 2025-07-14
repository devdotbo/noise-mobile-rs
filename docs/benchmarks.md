# Performance Benchmarks

## Overview

This document defines performance targets and benchmarking methodology for `noise-mobile-rust`. All benchmarks should be run on real mobile devices to ensure accuracy.

## Performance Targets

### Primary Metrics

| Operation | Target | Priority | Notes |
|-----------|--------|----------|-------|
| XX Handshake (3 messages) | < 10ms | Critical | Total time for complete handshake |
| Single Encryption (1KB) | < 1ms | Critical | ChaCha20-Poly1305 |
| Single Decryption (1KB) | < 1ms | Critical | Including authentication |
| Bulk Encryption (1MB) | > 10MB/s | High | Throughput test |
| Memory per Session | < 1KB | High | Excluding buffers |
| Session Creation | < 0.5ms | Medium | Including key generation |

### Mobile-Specific Targets

| Scenario | Target | Platform | Notes |
|----------|--------|----------|-------|
| BLE Handshake | < 500ms | iOS/Android | Including BLE overhead |
| Background->Foreground | < 50ms | iOS | Session restoration |
| Battery Impact | < 1% | Android | 1000 messages |
| Memory Pressure Response | < 10ms | iOS | Cleanup time |

## Benchmark Suite

### Setup

```rust
// benches/setup.rs
use noise_mobile::{NoiseSession, NoiseMode};
use criterion::{BatchSize, Bencher};

pub fn create_test_message(size: usize) -> Vec<u8> {
    vec![0xAB; size]
}

pub fn create_connected_pair() -> (NoiseSession, NoiseSession) {
    let mut initiator = NoiseSession::new(NoiseMode::Initiator).unwrap();
    let mut responder = NoiseSession::new(NoiseMode::Responder).unwrap();
    
    // Complete handshake
    let msg1 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg1).unwrap();
    
    let msg2 = responder.write_message(&[]).unwrap();
    initiator.read_message(&msg2).unwrap();
    
    let msg3 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg3).unwrap();
    
    (initiator, responder)
}
```

### Handshake Benchmarks

```rust
// benches/handshake.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_xx_handshake(c: &mut Criterion) {
    c.bench_function("noise_xx_complete_handshake", |b| {
        b.iter(|| {
            let mut initiator = NoiseSession::new(NoiseMode::Initiator).unwrap();
            let mut responder = NoiseSession::new(NoiseMode::Responder).unwrap();
            
            let msg1 = initiator.write_message(&[]).unwrap();
            responder.read_message(&msg1).unwrap();
            
            let msg2 = responder.write_message(&[]).unwrap();
            initiator.read_message(&msg2).unwrap();
            
            let msg3 = initiator.write_message(&[]).unwrap();
            responder.read_message(&msg3).unwrap();
            
            black_box((initiator, responder))
        });
    });
}

fn bench_handshake_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("handshake_messages");
    
    // Message 1: e (32 bytes)
    group.bench_function("message_1_write", |b| {
        b.iter_batched(
            || NoiseSession::new(NoiseMode::Initiator).unwrap(),
            |mut session| black_box(session.write_message(&[])),
            BatchSize::SmallInput,
        );
    });
    
    // Message 2: e, ee, s, es (96 bytes)
    group.bench_function("message_2_write", |b| {
        b.iter_batched(
            || {
                let mut init = NoiseSession::new(NoiseMode::Initiator).unwrap();
                let mut resp = NoiseSession::new(NoiseMode::Responder).unwrap();
                let msg1 = init.write_message(&[]).unwrap();
                resp.read_message(&msg1).unwrap();
                resp
            },
            |mut session| black_box(session.write_message(&[])),
            BatchSize::SmallInput,
        );
    });
    
    group.finish();
}
```

### Encryption Benchmarks

```rust
// benches/encryption.rs
use criterion::{BenchmarkId, Throughput};

fn bench_encryption_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption");
    
    for size in [64, 256, 1024, 4096, 16384, 65536].iter() {
        let data = create_test_message(*size);
        
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &data,
            |b, data| {
                let (mut session, _) = create_connected_pair();
                b.iter(|| {
                    black_box(session.encrypt(data).unwrap())
                });
            },
        );
    }
    
    group.finish();
}

fn bench_bulk_encryption(c: &mut Criterion) {
    c.bench_function("bulk_encrypt_1mb", |b| {
        let (mut session, _) = create_connected_pair();
        let data = create_test_message(1024 * 1024); // 1MB
        
        b.iter(|| {
            black_box(session.encrypt(&data).unwrap())
        });
    });
}

fn bench_message_stream(c: &mut Criterion) {
    c.bench_function("message_stream_1000x256b", |b| {
        let (mut alice, mut bob) = create_connected_pair();
        let messages: Vec<_> = (0..1000)
            .map(|_| create_test_message(256))
            .collect();
        
        b.iter(|| {
            for msg in &messages {
                let encrypted = alice.encrypt(msg).unwrap();
                black_box(bob.decrypt(&encrypted).unwrap());
            }
        });
    });
}
```

### Mobile Scenario Benchmarks

```rust
// benches/mobile_scenarios.rs

fn bench_ble_simulation(c: &mut Criterion) {
    c.bench_function("ble_handshake_simulation", |b| {
        b.iter(|| {
            let mut central = NoiseSession::new(NoiseMode::Initiator).unwrap();
            let mut peripheral = NoiseSession::new(NoiseMode::Responder).unwrap();
            
            // Simulate BLE MTU constraints (20-512 bytes)
            let mtu = 185; // Common BLE MTU
            
            // Message 1
            let msg1 = central.write_message(&[]).unwrap();
            assert!(msg1.len() <= mtu);
            peripheral.read_message(&msg1).unwrap();
            
            // Message 2 (might need fragmentation)
            let msg2 = peripheral.write_message(&[]).unwrap();
            if msg2.len() <= mtu {
                central.read_message(&msg2).unwrap();
            } else {
                // Simulate fragmentation
                for chunk in msg2.chunks(mtu) {
                    // In reality, these would be sent separately
                }
                central.read_message(&msg2).unwrap();
            }
            
            // Message 3
            let msg3 = central.write_message(&[]).unwrap();
            peripheral.read_message(&msg3).unwrap();
            
            black_box((central, peripheral))
        });
    });
}

fn bench_battery_scenario(c: &mut Criterion) {
    c.bench_function("battery_1000_messages", |b| {
        let (mut alice, mut bob) = create_connected_pair();
        
        b.iter(|| {
            // Simulate typical chat session
            for i in 0..1000 {
                let msg = format!("Message {}", i);
                let encrypted = alice.encrypt(msg.as_bytes()).unwrap();
                bob.decrypt(&encrypted).unwrap();
                
                // Simulate response
                if i % 3 == 0 {
                    let response = format!("Reply to {}", i);
                    let encrypted = bob.encrypt(response.as_bytes()).unwrap();
                    alice.decrypt(&encrypted).unwrap();
                }
            }
        });
    });
}

fn bench_memory_pressure(c: &mut Criterion) {
    c.bench_function("session_memory_overhead", |b| {
        b.iter(|| {
            let sessions: Vec<_> = (0..100)
                .map(|_| NoiseSession::new(NoiseMode::Initiator).unwrap())
                .collect();
            
            black_box(sessions)
        });
    });
}
```

### FFI Overhead Benchmarks

```rust
// benches/ffi_overhead.rs
use std::ffi::c_int;

fn bench_ffi_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("ffi_overhead");
    
    // Measure Rust native vs FFI call overhead
    group.bench_function("native_encrypt", |b| {
        let (mut session, _) = create_connected_pair();
        let data = create_test_message(1024);
        
        b.iter(|| {
            black_box(session.encrypt(&data).unwrap())
        });
    });
    
    group.bench_function("ffi_encrypt", |b| {
        unsafe {
            let mut error: c_int = 0;
            let session = noise_session_new(0, &mut error);
            // Complete handshake via FFI...
            
            let data = create_test_message(1024);
            let mut output = vec![0u8; 1024 + 16];
            let mut output_len = output.len();
            
            b.iter(|| {
                noise_encrypt(
                    session,
                    data.as_ptr(),
                    data.len(),
                    output.as_mut_ptr(),
                    &mut output_len,
                );
                black_box(&output[..output_len])
            });
            
            noise_session_free(session);
        }
    });
    
    group.finish();
}
```

## Platform-Specific Benchmarks

### iOS Benchmarks

```swift
// benchmarks/ios/NoisePerformanceTests.swift
import XCTest

class NoisePerformanceTests: XCTestCase {
    func testHandshakePerformance() {
        measure {
            let initiator = try! NoiseSession(mode: .initiator)
            let responder = try! NoiseSession(mode: .responder)
            
            let msg1 = try! initiator.writeMessage()
            _ = try! responder.readMessage(msg1)
            
            let msg2 = try! responder.writeMessage()
            _ = try! initiator.readMessage(msg2)
            
            let msg3 = try! initiator.writeMessage()
            _ = try! responder.readMessage(msg3)
        }
    }
    
    func testBLEIntegrationPerformance() {
        let options = XCTMeasureOptions()
        options.iterationCount = 100
        
        measure(options: options) {
            // Simulate BLE communication with real CoreBluetooth
            // This would use actual BLE hardware on device
        }
    }
    
    func testMemoryUsage() {
        let options = XCTMeasureOptions()
        options.iterationCount = 10
        
        measure(metrics: [XCTMemoryMetric()], options: options) {
            var sessions: [NoiseSession] = []
            for _ in 0..<100 {
                sessions.append(try! NoiseSession(mode: .initiator))
            }
            // Force deallocation
            sessions.removeAll()
        }
    }
}
```

### Android Benchmarks

```kotlin
// benchmarks/android/NoiseBenchmark.kt
@RunWith(AndroidJUnit4::class)
class NoiseBenchmark {
    @get:Rule
    val benchmarkRule = BenchmarkRule()
    
    @Test
    fun benchmarkHandshake() {
        benchmarkRule.measureRepeated {
            val initiator = NoiseSession(NoiseSession.Mode.INITIATOR)
            val responder = NoiseSession(NoiseSession.Mode.RESPONDER)
            
            val msg1 = initiator.writeMessage()
            responder.readMessage(msg1)
            
            val msg2 = responder.writeMessage()
            initiator.readMessage(msg2)
            
            val msg3 = initiator.writeMessage()
            responder.readMessage(msg3)
            
            // Cleanup
            initiator.close()
            responder.close()
        }
    }
    
    @Test
    fun benchmarkBulkEncryption() {
        val (alice, bob) = createConnectedPair()
        val data = ByteArray(1024 * 1024) // 1MB
        
        benchmarkRule.measureRepeated {
            alice.encrypt(data)
        }
    }
    
    @Test
    fun benchmarkBatteryImpact() {
        // This would use Android Battery Historian
        val powerManager = context.getSystemService(Context.POWER_SERVICE) as PowerManager
        val startBattery = powerManager.batteryLevel
        
        // Run 1000 encryptions
        val (alice, bob) = createConnectedPair()
        repeat(1000) {
            val encrypted = alice.encrypt("Test message".toByteArray())
            bob.decrypt(encrypted)
        }
        
        val endBattery = powerManager.batteryLevel
        assertTrue("Battery drain < 1%", startBattery - endBattery < 1)
    }
}
```

## Benchmark Results Template

### Hardware Specifications
- Device Model: [e.g., iPhone 13 Pro]
- OS Version: [e.g., iOS 16.0]
- CPU: [e.g., A15 Bionic]
- RAM: [e.g., 6GB]
- Test Date: [YYYY-MM-DD]

### Results Summary

| Benchmark | Target | Result | Status |
|-----------|--------|--------|--------|
| XX Handshake | < 10ms | X.Xms | ✓/✗ |
| 1KB Encrypt | < 1ms | X.Xms | ✓/✗ |
| 1KB Decrypt | < 1ms | X.Xms | ✓/✗ |
| 1MB Throughput | > 10MB/s | XX.XMB/s | ✓/✗ |
| Memory/Session | < 1KB | XXX bytes | ✓/✗ |

### Detailed Results

```
test bench_xx_handshake          ... bench:       X,XXX ns/iter (+/- XXX)
test bench_encrypt_1kb           ... bench:         XXX ns/iter (+/- XX)
test bench_decrypt_1kb           ... bench:         XXX ns/iter (+/- XX)
test bench_bulk_encrypt_1mb      ... bench:   X,XXX,XXX ns/iter (+/- XX,XXX)
test bench_session_memory        ... bench:       X,XXX ns/iter (+/- XXX)
```

## Optimization Strategies

### If Handshake Too Slow
1. Profile with `cargo flamegraph`
2. Check for unnecessary allocations
3. Ensure release mode optimizations
4. Consider pre-computed values

### If Encryption Too Slow
1. Verify SIMD optimizations enabled
2. Check alignment of buffers
3. Profile cache misses
4. Consider hardware acceleration

### If Memory Too High
1. Review buffer allocation strategy
2. Check for memory leaks with Valgrind
3. Use more compact data structures
4. Implement lazy initialization

## Continuous Benchmarking

### CI Integration
```yaml
# .github/workflows/benchmark.yml
name: Benchmarks

on:
  pull_request:
    types: [opened, synchronize]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Run benchmarks
        run: cargo bench --bench '*' -- --save-baseline pr
        
      - name: Compare with main
        run: |
          git checkout main
          cargo bench --bench '*' -- --save-baseline main
          cargo bench --bench '*' -- --load-baseline main --baseline pr
```

### Performance Regression Detection
- Set up criterion.rs with baseline saving
- Compare PR benchmarks against main
- Fail CI if regression > 5%
- Track trends over time

## Mobile-Specific Testing

### Real Device Requirements
1. Disable CPU throttling
2. Ensure consistent temperature
3. Close background apps
4. Use release builds only
5. Test multiple device models

### BLE Performance Testing
- Measure with real BLE hardware
- Test different MTU sizes
- Account for connection intervals
- Test with interference
- Measure battery drain

### Platform Comparison

| Platform | Handshake | 1KB Encrypt | 1MB Throughput |
|----------|-----------|-------------|----------------|
| iOS (A15) | X.Xms | X.Xms | XX.XMB/s |
| Android (SD888) | X.Xms | X.Xms | XX.XMB/s |
| iOS Simulator | X.Xms | X.Xms | XX.XMB/s |
| Android Emulator | X.Xms | X.Xms | XX.XMB/s |

Note: Always prefer real device results over simulator/emulator