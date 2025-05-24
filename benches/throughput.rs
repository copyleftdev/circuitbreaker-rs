use circuitbreaker_rs::{CircuitBreaker, DefaultPolicy};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::error::Error;
use std::fmt;
use std::time::Duration;

// Custom error type that implements Error trait
#[derive(Debug)]
struct BenchError(String);

impl BenchError {
    fn new(msg: &str) -> Self {
        BenchError(msg.to_string())
    }
}

impl fmt::Display for BenchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Benchmark error: {}", self.0)
    }
}

impl Error for BenchError {}

fn successful_operation() -> Result<(), BenchError> {
    Ok(())
}

fn failing_operation() -> Result<(), BenchError> {
    Err(BenchError::new("Simulated failure"))
}

fn bench_circuit_breaker_closed(c: &mut Criterion) {
    let breaker = CircuitBreaker::<DefaultPolicy, BenchError>::builder()
        .failure_threshold(0.5)
        .cooldown(Duration::from_secs(30))
        .build();

    c.bench_function("circuit_breaker_closed_success", |b| {
        b.iter(|| black_box(breaker.call(successful_operation)));
    });
}

fn bench_circuit_breaker_transition(c: &mut Criterion) {
    let breaker = CircuitBreaker::<DefaultPolicy, BenchError>::builder()
        .failure_threshold(0.5)
        .consecutive_failures(5)
        .cooldown(Duration::from_secs(30))
        .build();

    c.bench_function("circuit_breaker_transition", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();

            for _ in 0..iters {
                // Force closed to ensure consistent starting point
                breaker.force_closed();

                // Make 5 failing calls to trip the breaker
                for _ in 0..6 {
                    let _ = black_box(breaker.call(failing_operation));
                }

                // One open-circuit rejection
                let _ = black_box(breaker.call(successful_operation));
            }

            start.elapsed()
        });
    });
}

fn bench_circuit_breaker_concurrent(c: &mut Criterion) {
    use std::sync::{Arc, Barrier};
    use std::thread;

    let breaker = Arc::new(
        CircuitBreaker::<DefaultPolicy, BenchError>::builder()
            .failure_threshold(0.5)
            .consecutive_failures(100) // High to avoid tripping
            .cooldown(Duration::from_secs(30))
            .build(),
    );

    const THREAD_COUNT: usize = 4;
    const ITERATIONS_PER_THREAD: usize = 1000;

    c.bench_function("circuit_breaker_concurrent", |b| {
        b.iter(|| {
            let barrier = Arc::new(Barrier::new(THREAD_COUNT + 1));
            let mut handles = Vec::with_capacity(THREAD_COUNT);

            for _ in 0..THREAD_COUNT {
                let thread_breaker = Arc::clone(&breaker);
                let thread_barrier = Arc::clone(&barrier);

                handles.push(thread::spawn(move || {
                    thread_barrier.wait();
                    for _ in 0..ITERATIONS_PER_THREAD {
                        let _ = black_box(thread_breaker.call(successful_operation));
                    }
                }));
            }

            // Start all threads simultaneously
            barrier.wait();

            // Wait for all threads to complete
            for handle in handles {
                handle.join().unwrap();
            }
        });
    });
}

criterion_group!(
    benches,
    bench_circuit_breaker_closed,
    bench_circuit_breaker_transition,
    bench_circuit_breaker_concurrent
);
criterion_main!(benches);
