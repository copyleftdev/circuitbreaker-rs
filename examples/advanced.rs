//! Advanced Circuit Breaker Example
//!
//! This example demonstrates:
//! 1. Creating a custom error type
//! 2. Creating a custom circuit breaker policy
//! 3. Using hooks for monitoring circuit breaker events
//! 4. Handling different circuit breaker states

use circuitbreaker_rs::{BreakerError, CircuitBreaker, DefaultPolicy, HookRegistry};
use std::error::Error;
use std::fmt;
use std::thread;
use std::time::Duration;

// Custom error type that implements Error trait
#[derive(Debug)]
struct ServiceError(String);

impl ServiceError {
    fn new(msg: &str) -> Self {
        ServiceError(msg.to_string())
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Service error: {}", self.0)
    }
}

impl Error for ServiceError {}

// A function that simulates an external service with varying failure patterns
fn external_service_call(fail_count: &mut u32) -> Result<String, ServiceError> {
    *fail_count += 1;

    // For demonstration: fail on specific patterns
    if *fail_count <= 3 {
        // First 3 calls succeed
        Ok("Initial success".to_string())
    } else if *fail_count <= 8 {
        // Next 5 calls fail (should trip the breaker)
        Err(ServiceError::new("Service temporarily unavailable"))
    } else if *fail_count <= 10 {
        // Next 2 calls succeed (when the breaker transitions to half-open)
        Ok("Service recovered".to_string())
    } else {
        // After that, all calls succeed
        Ok("Stable success".to_string())
    }
}

fn main() {
    println!("=== Advanced Circuit Breaker Example ===\n");

    // 1. Set up a hook registry for observability
    let hooks = HookRegistry::new();

    hooks.set_on_open(|| println!("📢 Circuit OPENED due to too many failures"));
    hooks.set_on_close(|| println!("📢 Circuit CLOSED after successful recovery"));
    hooks.set_on_half_open(|| println!("📢 Circuit HALF-OPEN, testing if service recovered"));

    hooks.set_on_success(|| println!("✅ Call succeeded"));
    hooks.set_on_failure(|| println!("❌ Call failed"));

    // 2. Create a circuit breaker with advanced configuration
    let breaker = CircuitBreaker::<DefaultPolicy, ServiceError>::builder()
        .failure_threshold(0.5) // Trip when error rate exceeds 50%
        .consecutive_failures(3) // Or trip after 3 consecutive failures
        .consecutive_successes(2) // Reset after 2 consecutive successes when half-open
        .cooldown(Duration::from_secs(2)) // Wait 2 seconds before trying half-open state
        .hooks(hooks) // Add our hooks
        .build();

    println!("Initial state: {:?}\n", breaker.current_state());

    // 3. Simulate a series of calls to demonstrate the circuit breaker behavior
    let mut fail_count = 0;

    for i in 1..=15 {
        println!("\n--- Call {} ---", i);

        // Make the call through the circuit breaker
        let result = breaker.call(|| external_service_call(&mut fail_count));

        match result {
            Ok(response) => println!("🔄 Service response: {}", response),
            Err(BreakerError::Open) => println!("🔄 Circuit open, call not attempted"),
            Err(BreakerError::Operation(err)) => println!("🔄 Service error: {}", err),
            Err(err) => println!("🔄 Other error: {}", err),
        }

        // Print circuit breaker state and error rate
        println!(
            "Circuit metrics: state={:?}, error_rate={:.2}",
            breaker.current_state(),
            breaker.error_rate()
        );

        // Add a delay between calls for readability
        thread::sleep(Duration::from_millis(500));
    }

    println!("\n=== Example Completed ===");
}
