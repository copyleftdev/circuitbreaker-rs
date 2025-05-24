//! # circuitbreaker-rs
//!
//! A production-grade, zero-boilerplate, lock-efficient, observability-ready
//! Circuit Breaker library for Rust applications.
//!
//! This library provides concurrent-safe circuit breaker functionality with both
//! sync and async interfaces, designed for performance-critical systems.
//!
//! ## What is a Circuit Breaker?
//!
//! The Circuit Breaker pattern helps prevent cascading failures in distributed systems
//! by temporarily disabling operations that are likely to fail. This pattern is inspired
//! by electrical circuit breakers and operates in three states:
//!
//! - **Closed**: Normal operation. Calls pass through to the protected resource.
//! - **Open**: Calls are immediately rejected without attempting to reach the resource.
//! - **Half-Open**: After a cooldown period, a limited number of test calls are permitted
//!   to check if the underlying resource has recovered.
//!
//! ## Basic Usage
//!
//! ```rust
//! use circuitbreaker_rs::{CircuitBreaker, BreakerError, DefaultPolicy};
//! use std::error::Error;
//! use std::fmt;
//! use std::time::Duration;
//!
//! // Define a custom error type that implements Error trait
//! #[derive(Debug)]
//! struct ServiceError(String);
//!
//! impl fmt::Display for ServiceError {
//!     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//!         write!(f, "Service error: {}", self.0)
//!     }
//! }
//!
//! impl Error for ServiceError {}
//!
//! // Create a circuit breaker with custom settings
//! let breaker = CircuitBreaker::<DefaultPolicy, ServiceError>::builder()
//!     .failure_threshold(0.5) // Trip when 50% of calls fail
//!     .cooldown(Duration::from_secs(30)) // Wait 30 seconds before trying to recover
//!     .build();
//!
//! // Use the circuit breaker to wrap function calls
//! match breaker.call(|| {
//!     // Your service call that might fail
//!     Ok("Success".to_string()) // Simulate success
//!     // Err(ServiceError("Service unavailable".to_string())) // Or simulate failure
//! }) {
//!     Ok(result) => println!("Call succeeded: {}", result),
//!     Err(BreakerError::Open) => println!("Circuit is open, call was prevented"),
//!     Err(BreakerError::Operation(err)) => println!("Call failed: {}", err),
//!     Err(err) => println!("Other error: {}", err),
//! }
//! ```
//!
//! ## Async Support
//!
//! With the `async` feature enabled, you can use the circuit breaker with async operations:
//!
//! ```rust,ignore
//! // Enable the "async" feature in Cargo.toml
//! let breaker = CircuitBreaker::<DefaultPolicy, ServiceError>::builder().build();
//!
//! let result = breaker.call_async(|| async {
//!     // Your async service call
//!     Ok("Success".to_string())
//! }).await;
//! ```
//!
//! ## Features
//!
//! - `std` - Standard library support (default)
//! - `async` - Async support with Tokio
//! - `prometheus` - Prometheus metrics integration
//! - `tracing` - Tracing integration

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod breaker;
mod config;
mod error;
mod hook;
mod metrics;
mod policy;
pub mod prelude;
mod state;

// Re-exports
pub use breaker::CircuitBreaker;
pub use config::BreakerBuilder;
pub use error::{BreakerError, BreakerResult};
pub use hook::HookRegistry;
pub use metrics::{EMAWindow, FixedWindow, MetricSink};
pub use policy::{BreakerPolicy, DefaultPolicy, ThroughputAwarePolicy, TimeBasedPolicy};
pub use state::State;
