use circuitbreaker_rs::{BreakerError, CircuitBreaker, DefaultPolicy};
use std::error::Error;
use std::fmt;
use std::thread;
use std::time::Duration;

// Custom error type that implements Error trait
#[derive(Debug)]
struct ServiceError(String);

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Service error: {}", self.0)
    }
}

impl Error for ServiceError {}

fn main() {
    // Create a circuit breaker with default settings
    let breaker = CircuitBreaker::<DefaultPolicy, ServiceError>::builder()
        .failure_threshold(0.5) // 50% failure rate will trip circuit
        .cooldown(Duration::from_secs(5)) // 5 second cooldown period
        .probe_interval(3) // Allow 3 test requests when half-open
        .build();

    println!("Circuit initial state: {:?}", breaker.current_state());

    // Create a mutable counter for tracking failures
    let mut fail_counter = 0;

    // Make calls with a function that creates a new closure each time to avoid the move issue
    let call_service = |counter: &mut u32| -> Result<String, ServiceError> {
        if *counter < 10 {
            *counter += 1;
            if *counter % 2 == 0 {
                // Simulate an error on even counts
                Err(ServiceError("External service error".to_string()))
            } else {
                Ok("Success".to_string())
            }
        } else {
            // After 10 calls, start succeeding to demonstrate recovery
            Ok("Success".to_string())
        }
    };

    // Make 15 calls with the circuit breaker
    for i in 1..=15 {
        println!("\nAttempt {}: ", i);

        // Use the call_service function with our counter
        match breaker.call(|| call_service(&mut fail_counter)) {
            Ok(result) => println!("Call succeeded with result: {}", result),
            Err(BreakerError::Open) => {
                println!("Circuit is open, waiting before retry...");
                thread::sleep(Duration::from_secs(1));
            }
            Err(BreakerError::Operation(err)) => {
                println!("Call failed with error: {}", err);
            }
            Err(err) => println!("Other error: {}", err),
        }

        println!(
            "Current state: {:?}, Error rate: {:.2}",
            breaker.current_state(),
            breaker.error_rate()
        );

        // Add a small delay between calls
        thread::sleep(Duration::from_millis(300));
    }
}
