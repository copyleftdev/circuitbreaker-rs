use circuitbreaker_rs::{
    BreakerBuilder, BreakerError, BreakerPolicy, BreakerStats, CircuitBreaker, DefaultPolicy,
    HookRegistry, State,
};
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

// Custom policy that implements BreakerPolicy trait
struct CustomPolicy {
    consecutive_failures_threshold: u64,
    consecutive_successes_threshold: u64,
    error_rate_threshold: f64,
}

impl CustomPolicy {
    fn new(
        consecutive_failures_threshold: u64,
        consecutive_successes_threshold: u64,
        error_rate_threshold: f64,
    ) -> Self {
        Self {
            consecutive_failures_threshold,
            consecutive_successes_threshold,
            error_rate_threshold,
        }
    }
}

impl BreakerPolicy for CustomPolicy {
    fn should_trip(&self, stats: &BreakerStats) -> bool {
        // Trip if consecutive failures exceed threshold OR error rate is too high
        stats.consecutive_failures() >= self.consecutive_failures_threshold
            || (stats.total_calls() > 10 && stats.error_rate() >= self.error_rate_threshold)
    }

    fn should_reset(&self, stats: &BreakerStats) -> bool {
        // Reset if we've had enough consecutive successes in half-open state
        stats.consecutive_successes() >= self.consecutive_successes_threshold
    }
}

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
    let mut hooks = HookRegistry::new();
    
    hooks.set_on_open(|| println!("📢 Circuit OPENED due to too many failures"));
    hooks.set_on_close(|| println!("📢 Circuit CLOSED after successful recovery"));
    hooks.set_on_half_open(|| println!("📢 Circuit HALF-OPEN, testing if service recovered"));
    
    hooks.set_on_success(|| println!("✅ Call succeeded"));
    hooks.set_on_failure(|_| println!("❌ Call failed"));
    hooks.set_on_rejected(|| println!("🚫 Call rejected (circuit open)"));
    
    // 2. Create a circuit breaker with custom policy
    let custom_policy = CustomPolicy::new(
        3,     // Trip after 3 consecutive failures
        2,     // Reset after 2 consecutive successes when half-open
        0.5,   // Or trip when error rate exceeds 50%
    );
    
    let breaker: CircuitBreaker<CustomPolicy, ServiceError> = BreakerBuilder::default()
        .policy(custom_policy)
        .cooldown(Duration::from_secs(2))  // Short cooldown for demonstration
        .hooks(hooks)
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
        
        // Print circuit breaker metrics
        println!(
            "Circuit metrics: state={:?}, error_rate={:.2}, consecutive_failures={}, consecutive_successes={}",
            breaker.current_state(),
            breaker.error_rate(),
            breaker.consecutive_failures(),
            breaker.consecutive_successes()
        );
        
        // Add a delay between calls for readability
        thread::sleep(Duration::from_millis(500));
    }
    
    println!("\n=== Example Completed ===");
}
