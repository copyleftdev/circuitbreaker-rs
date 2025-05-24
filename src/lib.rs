//! # circuitbreaker-rs
//!
//! A production-grade, zero-boilerplate, lock-efficient, observability-ready
//! Circuit Breaker library for Rust applications.
//!
//! This library provides concurrent-safe circuit breaker functionality with both
//! sync and async interfaces, designed for performance-critical systems.

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
