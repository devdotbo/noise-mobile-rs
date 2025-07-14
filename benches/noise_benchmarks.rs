use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use noise_mobile::core::{
    session::NoiseSession,
    error::NoiseError,
};
use noise_mobile::mobile::{
    battery::BatchedCrypto,
    network::ResilientSession,
};
use noise_mobile::ffi::c_api;
use std::time::Duration;

/// Helper function to create a connected pair of sessions
fn create_connected_pair() -> Result<(NoiseSession, NoiseSession), NoiseError> {
    let mut initiator = NoiseSession::new_initiator()?;
    let mut responder = NoiseSession::new_responder()?;
    
    // Perform full XX handshake
    // Message 1: initiator -> responder (e)
    let msg1 = initiator.write_message(&[])?;
    responder.read_message(&msg1)?;
    
    // Message 2: responder -> initiator (e, ee, s, es)
    let msg2 = responder.write_message(&[])?;
    initiator.read_message(&msg2)?;
    
    // Message 3: initiator -> responder (s, se)
    let msg3 = initiator.write_message(&[])?;
    responder.read_message(&msg3)?;
    
    // Both should now be in transport mode
    if !initiator.is_transport_state() || !responder.is_transport_state() {
        return Err(NoiseError::InvalidState("Sessions not in transport mode".to_string()));
    }
    
    Ok((initiator, responder))
}

/// Benchmark the full Noise_XX handshake
fn benchmark_handshake(c: &mut Criterion) {
    c.bench_function("noise_xx_handshake", |b| {
        b.iter(|| {
            let mut initiator = NoiseSession::new_initiator().unwrap();
            let mut responder = NoiseSession::new_responder().unwrap();
            
            // Message 1
            let msg1 = initiator.write_message(&[]).unwrap();
            responder.read_message(&msg1).unwrap();
            
            // Message 2
            let msg2 = responder.write_message(&[]).unwrap();
            initiator.read_message(&msg2).unwrap();
            
            // Message 3
            let msg3 = initiator.write_message(&[]).unwrap();
            responder.read_message(&msg3).unwrap();
            
            black_box((initiator, responder))
        })
    });
}

/// Benchmark handshake with payload in each message
fn benchmark_handshake_with_payload(c: &mut Criterion) {
    let payload = b"Hello, this is a test payload for handshake messages";
    
    c.bench_function("noise_xx_handshake_with_payload", |b| {
        b.iter(|| {
            let mut initiator = NoiseSession::new_initiator().unwrap();
            let mut responder = NoiseSession::new_responder().unwrap();
            
            // Message 1 with payload
            let msg1 = initiator.write_message(payload).unwrap();
            let _payload1 = responder.read_message(&msg1).unwrap();
            
            // Message 2 with payload
            let msg2 = responder.write_message(payload).unwrap();
            let _payload2 = initiator.read_message(&msg2).unwrap();
            
            // Message 3 with payload
            let msg3 = initiator.write_message(payload).unwrap();
            let _payload3 = responder.read_message(&msg3).unwrap();
            
            black_box((initiator, responder))
        })
    });
}

/// Benchmark encryption throughput with various message sizes
fn benchmark_encryption_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption_throughput");
    
    // Test various message sizes (bytes)
    let sizes = [
        64,      // Small message
        512,     // Typical chat message
        1024,    // 1KB
        4096,    // 4KB
        8192,    // 8KB
        16384,   // 16KB
        32768,   // 32KB
        65519,   // Max message size (65535 - 16 byte overhead)
    ];
    
    for size in sizes.iter() {
        let (mut initiator, _responder) = create_connected_pair().unwrap();
        let data = vec![0x42u8; *size]; // Fill with dummy data
        
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, _| {
                b.iter(|| {
                    let encrypted = initiator.encrypt(&data).unwrap();
                    black_box(encrypted)
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark decryption throughput
fn benchmark_decryption_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("decryption_throughput");
    
    let sizes = [64, 512, 1024, 4096, 8192, 16384, 32768, 65519];
    
    for size in sizes.iter() {
        let data = vec![0x42u8; *size];
        
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, _| {
                // Create fresh sessions for each iteration to avoid out-of-order issues
                let (mut initiator, mut responder) = create_connected_pair().unwrap();
                
                b.iter(|| {
                    let encrypted = initiator.encrypt(&data).unwrap();
                    let decrypted = responder.decrypt(&encrypted).unwrap();
                    black_box(decrypted)
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark batch vs individual crypto operations
fn benchmark_batch_vs_individual(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_vs_individual");
    
    let message_counts = [5, 10, 20, 50];
    
    for count in message_counts.iter() {
        let messages: Vec<Vec<u8>> = (0..*count)
            .map(|i| format!("Test message #{} with some content to make it realistic", i).into_bytes())
            .collect();
        
        // Individual operations benchmark
        group.bench_with_input(
            BenchmarkId::new("individual", count),
            count,
            |b, _| {
                b.iter(|| {
                    let (mut session, _) = create_connected_pair().unwrap();
                    let mut results = Vec::with_capacity(messages.len());
                    
                    for msg in &messages {
                        results.push(session.encrypt(msg).unwrap());
                    }
                    
                    black_box(results)
                })
            }
        );
        
        // Batched operations benchmark
        group.bench_with_input(
            BenchmarkId::new("batched", count),
            count,
            |b, _| {
                b.iter(|| {
                    let (session, _) = create_connected_pair().unwrap();
                    let mut batched = BatchedCrypto::new(session);
                    
                    for msg in &messages {
                        batched.queue_encrypt(msg.clone());
                    }
                    
                    let results = batched.flush_encrypts().unwrap();
                    black_box(results)
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark resilient session with replay protection
fn benchmark_resilient_session(c: &mut Criterion) {
    let mut group = c.benchmark_group("resilient_session");
    
    // Benchmark encryption with sequence numbers
    group.bench_function("encrypt_with_sequence", |b| {
        let (initiator, _) = create_connected_pair().unwrap();
        let mut resilient = ResilientSession::new(initiator);
        let data = b"Test message for resilient session";
        
        b.iter(|| {
            let encrypted = resilient.encrypt_with_sequence(data).unwrap();
            black_box(encrypted)
        })
    });
    
    // Benchmark decryption with replay check
    group.bench_function("decrypt_with_replay_check", |b| {
        let data = b"Test message for resilient session";
        
        b.iter(|| {
            let (initiator, responder) = create_connected_pair().unwrap();
            let mut resilient_sender = ResilientSession::new(initiator);
            let mut resilient_receiver = ResilientSession::new(responder);
            
            let encrypted = resilient_sender.encrypt_with_sequence(data).unwrap();
            let decrypted = resilient_receiver.decrypt_with_replay_check(&encrypted).unwrap();
            black_box(decrypted)
        })
    });
    
    group.finish();
}

/// Benchmark session creation overhead
fn benchmark_session_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_creation");
    
    group.bench_function("new_initiator", |b| {
        b.iter(|| {
            let session = NoiseSession::new_initiator().unwrap();
            black_box(session)
        })
    });
    
    group.bench_function("new_responder", |b| {
        b.iter(|| {
            let session = NoiseSession::new_responder().unwrap();
            black_box(session)
        })
    });
    
    // Benchmark session creation with custom key
    let private_key = [0x42u8; 32]; // Dummy key for benchmarking
    
    group.bench_function("with_private_key_initiator", |b| {
        b.iter(|| {
            let session = NoiseSession::with_private_key(&private_key, true).unwrap();
            black_box(session)
        })
    });
    
    group.finish();
}

/// Benchmark FFI overhead
fn benchmark_ffi_overhead(c: &mut Criterion) {
    use noise_mobile::ffi::c_api::*;
    use std::ptr;
    
    let mut group = c.benchmark_group("ffi_overhead");
    
    // Benchmark FFI session creation
    group.bench_function("ffi_session_new", |b| {
        b.iter(|| {
            let mut error = 0;
            let session = unsafe { c_api::noise_session_new(c_api::NOISE_MODE_INITIATOR, &mut error) };
            assert_eq!(error, c_api::NOISE_ERROR_SUCCESS);
            unsafe { c_api::noise_session_free(session); }
            black_box(session)
        })
    });
    
    // Note: FFI encryption benchmark would require completing handshake
    // which is complex to do correctly in this context.
    // For now, focusing on session creation overhead is sufficient.
    
    group.finish();
}

// Configure and run all benchmarks
criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets = 
        benchmark_handshake,
        benchmark_handshake_with_payload,
        benchmark_encryption_throughput,
        benchmark_decryption_throughput,
        benchmark_batch_vs_individual,
        benchmark_resilient_session,
        benchmark_session_creation,
        benchmark_ffi_overhead
}

criterion_main!(benches);