# circuitbreaker-rs

[![Crates.io](https://img.shields.io/crates/v/circuitbreaker-rs.svg)](https://crates.io/crates/circuitbreaker-rs)
[![Documentation](https://docs.rs/circuitbreaker-rs/badge.svg)](https://docs.rs/circuitbreaker-rs)
[![License](https://img.shields.io/crates/l/circuitbreaker-rs.svg)](LICENSE)

A production-grade, zero-boilerplate, lock-efficient, observability-ready Circuit Breaker library for Rust applications. Fast, reliable, and well-tested.

Implemented by [copyleftdev](https://github.com/copyleftdev).

## Overview

`circuitbreaker-rs` is a high-performance circuit breaker implementation designed for integration into performance-critical Rust systems. It provides both synchronous and asynchronous interfaces, integrates with observability stacks, and supports custom policies, configurable time-windows, and recovery strategies.

### What is a Circuit Breaker?

The Circuit Breaker pattern is used to improve system resilience by detecting failures and preventing cascading failures throughout the system. It works like an electrical circuit breaker:

1. **Closed State (Normal Operation)**: Calls pass through the circuit breaker to the protected service.
2. **Open State (Failure Protection)**: When failures exceed a threshold, the circuit "trips" and calls are rejected without attempting to reach the service.
3. **Half-Open State (Testing Recovery)**: After a cooldown period, a limited number of test calls are allowed through to check if the service has recovered.

This pattern is essential for building resilient distributed systems, microservices, and applications that interact with external dependencies.

## Features

- **Lock-free State Management**: Enum-based FSM using atomic operations for state transitions
- **Flexible Failure Tracking**: Support for both fixed-window and exponential moving average (EMA) metrics
- **Customizable Policies**: Implement your own tripping and reset logic or use the provided policies
- **Sync and Async Support**: Works with both blocking and async code
- **Observability Ready**: Built-in support for metrics collection and hooks for state transitions
- **Zero-alloc Hot Path**: Optimized for performance in critical paths
- **Rich Error Handling**: Detailed error information without using panics
- **Thread-Safe**: Fully concurrent-safe for use in multi-threaded applications
- **Minimal Dependencies**: Small dependency footprint for fast compilation and minimal bloat
- **Feature Flags**: Pay only for what you use with opt-in features

## Quick Start

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
circuitbreaker-rs = "0.1.0"
```

Basic usage:

```rust
use circuitbreaker_rs::{CircuitBreaker, BreakerError, State, DefaultPolicy};
use std::time::Duration;
use std::error::Error;

fn main() {
    // Create a circuit breaker with custom settings
    let breaker = CircuitBreaker::builder()
        .failure_threshold(0.5)
        .cooldown(Duration::from_secs(30))
        .probe_interval(3)
        .build();

    // Use the circuit breaker to wrap function calls
    match breaker.call(|| external_service_call()) {
        Ok(result) => println!("Call succeeded: {:?}", result),
        Err(BreakerError::Open) => println!("Circuit is open, call was prevented"),
        Err(BreakerError::Operation(err)) => println!("Call failed: {:?}", err),
        Err(err) => println!("Other error: {:?}", err),
    }
}

// Your function that might fail - must implement std::error::Error
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct MyError(String);

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Service error: {}", self.0)
    }
}

impl Error for MyError {}

fn external_service_call() -> Result<String, MyError> {
    // Actual implementation
    Ok("Success".to_string())
    // Or for an error: Err(MyError("Service unavailable".to_string()))
}
```

## Error Handling Requirements

Important: The error type used with the circuit breaker must implement the `std::error::Error` trait. Using types like `String` as errors directly will not work. Here's a simple pattern for creating a compatible error type:

```rust
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct MyError(String);

impl MyError {
    fn new(msg: &str) -> Self {
        MyError(msg.to_string())
    }
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "My error: {}", self.0)
    }
}

impl Error for MyError {}
```

## Advanced Configuration

The library offers extensive configuration options through the builder pattern:

```rust
let breaker = CircuitBreaker::builder()
    .failure_threshold(0.5)          // Trip when error rate exceeds 50%
    .min_throughput(10)              // Require at least 10 calls before considering error rate
    .cooldown(Duration::from_secs(30)) // Wait 30 seconds before trying half-open state
    .probe_interval(3)               // Allow 3 test requests when half-open
    .consecutive_failures(5)         // Trip after 5 consecutive failures regardless of rate
    .consecutive_successes(2)        // Reset after 2 consecutive successes in half-open state
    .metric_sink(prometheus_sink())  // Use Prometheus for metrics
    .build();
```

## Custom Policies

Implement the `BreakerPolicy` trait to create custom circuit breaker policies:

```rust
use circuitbreaker_rs::{BreakerPolicy, BreakerStats};

struct MyCustomPolicy {
    // Your policy configuration
}

impl BreakerPolicy for MyCustomPolicy {
    fn should_trip(&self, stats: &BreakerStats) -> bool {
        // Your logic to determine when to trip the circuit
        stats.consecutive_failures() > 10 && stats.error_rate() > 0.3
    }
    
    fn should_reset(&self, stats: &BreakerStats) -> bool {
        // Your logic to determine when to reset the circuit
        stats.consecutive_successes() >= 5
    }
}
```

## Async Support

Async support is available with the `async` feature:

```toml
[dependencies]
circuitbreaker-rs = { version = "0.1.0", features = ["async"] }
```

And then use the `call_async` method:

```rust
let breaker = CircuitBreaker::builder().build();

let result = breaker.call_async(|| async {
    external_async_service_call().await
}).await;
```

## Observability

The library provides hooks for state transitions and metric collection:

```rust
let mut hooks = HookRegistry::new();
hooks.set_on_open(|| println!("Circuit opened!"));
hooks.set_on_close(|| println!("Circuit closed!"));

let breaker = CircuitBreaker::builder()
    .hooks(hooks)
    .metric_sink(MyMetricSink::new())
    .build();
```

## Features Flags

- `std` - Standard library support (default)
- `async` - Async support with Tokio
- `prometheus` - Prometheus metrics integration
- `tracing` - Tracing integration

## Performance

`circuitbreaker-rs` is designed for high-performance scenarios. Here are some benchmark results from the included benchmarks:

| Benchmark | Description | Performance |
|-----------|-------------|-------------|
| `circuit_breaker_closed_success` | Regular operation (circuit closed) | ~80 ns per call |
| `circuit_breaker_transition` | State transition performance | ~600 ns per transition |
| `circuit_breaker_concurrent` | Multi-threaded performance (4 threads) | ~530 μs for 4000 operations |

These benchmarks demonstrate the library's minimal overhead during normal operation and efficient state transitions, making it suitable for high-throughput systems and latency-sensitive applications.

### Performance Considerations

- **Lock Contention**: The library uses atomic operations and lock-free state transitions where possible to minimize contention.
- **Memory Usage**: Fixed minimal allocation with reusable structures.
- **Async Overhead**: Minimal async overhead when using the async feature, only paying for what you use.

Run the benchmarks yourself with: `cargo bench`

## API Documentation

For full API documentation, visit [docs.rs/circuitbreaker-rs](https://docs.rs/circuitbreaker-rs).

### Core Types

- `CircuitBreaker`: The main circuit breaker struct
- `BreakerBuilder`: Builder pattern for constructing a circuit breaker with custom settings
- `BreakerPolicy`: Trait for implementing custom tripping and recovery policies
- `BreakerError`: Error type returned when a call fails or is rejected
- `State`: Enum representing the possible states of the circuit breaker (Closed, Open, HalfOpen)

### Example Implementations

Check the [examples directory](./examples) for more complete examples.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

## Author

[copyleftdev](https://github.com/copyleftdev)
