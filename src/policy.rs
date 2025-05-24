//! Policy engine for circuit breaker trip and reset decisions.

use crate::metrics::{BreakerStats, EMAWindow, FixedWindow};
use std::time::Duration;

/// A policy that determines when to trip and reset a circuit breaker.
pub trait BreakerPolicy: Send + Sync + 'static {
    /// Determines if the circuit should trip open based on current stats.
    fn should_trip(&self, stats: &BreakerStats) -> bool;

    /// Determines if the circuit should reset to closed based on current stats.
    fn should_reset(&self, stats: &BreakerStats) -> bool;
}

/// Default policy implementation based on error rate and consecutive failures.
pub struct DefaultPolicy {
    failure_threshold: f64,
    min_throughput: u64,
    consecutive_failures_threshold: u64,
    consecutive_successes_threshold: u64,
}

impl DefaultPolicy {
    /// Creates a new default policy.
    pub fn new(
        failure_threshold: f64,
        min_throughput: u64,
        consecutive_failures_threshold: u64,
        consecutive_successes_threshold: u64,
    ) -> Self {
        Self {
            failure_threshold,
            min_throughput,
            consecutive_failures_threshold,
            consecutive_successes_threshold,
        }
    }
}

impl BreakerPolicy for DefaultPolicy {
    fn should_trip(&self, stats: &BreakerStats) -> bool {
        // Trip if error rate exceeds threshold and we have minimum throughput
        let error_rate = stats.error_rate();
        let total_calls = stats.get_total_calls();

        if total_calls >= self.min_throughput && error_rate >= self.failure_threshold {
            return true;
        }

        // Or if consecutive failures exceed threshold
        stats.consecutive_failures() >= self.consecutive_failures_threshold
    }

    fn should_reset(&self, stats: &BreakerStats) -> bool {
        stats.consecutive_successes() >= self.consecutive_successes_threshold
    }
}

/// Time-based policy that considers time windows for decisions.
pub struct TimeBasedPolicy {
    window: FixedWindow,
    failure_threshold: f64,
    min_call_count: u64,
    min_recovery_time: Duration,
    consecutive_successes_threshold: u64,
}

impl TimeBasedPolicy {
    /// Creates a new time-based policy.
    pub fn new(
        window_size: Duration,
        bucket_count: usize,
        failure_threshold: f64,
        min_call_count: u64,
        min_recovery_time: Duration,
        consecutive_successes_threshold: u64,
    ) -> Self {
        Self {
            window: FixedWindow::new(window_size, bucket_count),
            failure_threshold,
            min_call_count,
            min_recovery_time,
            consecutive_successes_threshold,
        }
    }

    /// Records a successful call in the time window.
    pub fn record_success(&self) {
        self.window.record_success();
    }

    /// Records a failed call in the time window.
    pub fn record_failure(&self) {
        self.window.record_failure();
    }
}

impl BreakerPolicy for TimeBasedPolicy {
    fn should_trip(&self, stats: &BreakerStats) -> bool {
        let window_error_rate = self.window.error_rate();
        let total_calls = stats.get_total_calls();

        window_error_rate >= self.failure_threshold && total_calls >= self.min_call_count
    }

    fn should_reset(&self, stats: &BreakerStats) -> bool {
        let last_failure = stats.get_last_failure_time();

        if let Some(time) = last_failure {
            if time.elapsed() < self.min_recovery_time {
                return false;
            }
        }

        stats.consecutive_successes() >= self.consecutive_successes_threshold
    }
}

/// Throughput-aware policy that uses EMA for error rate tracking.
pub struct ThroughputAwarePolicy {
    ema_window: EMAWindow,
    failure_threshold: f64,
    min_throughput_per_second: f64,
    throughput_window: Duration,
    recovery_threshold: f64,
}

impl ThroughputAwarePolicy {
    /// Creates a new throughput-aware policy.
    pub fn new(
        alpha: f64,
        calls_required: u64,
        failure_threshold: f64,
        min_throughput_per_second: f64,
        throughput_window: Duration,
        recovery_threshold: f64,
    ) -> Self {
        Self {
            ema_window: EMAWindow::new(alpha, calls_required),
            failure_threshold,
            min_throughput_per_second,
            throughput_window,
            recovery_threshold,
        }
    }

    /// Records a successful call in the EMA window.
    pub fn record_success(&self) {
        self.ema_window.record_success();
    }

    /// Records a failed call in the EMA window.
    pub fn record_failure(&self) {
        self.ema_window.record_failure();
    }

    fn calculate_throughput(&self, stats: &BreakerStats) -> f64 {
        let total_calls = stats.get_total_calls();

        let window_secs = self.throughput_window.as_secs_f64();
        if window_secs <= 0.0 {
            return 0.0;
        }

        total_calls as f64 / window_secs
    }
}

impl BreakerPolicy for ThroughputAwarePolicy {
    fn should_trip(&self, stats: &BreakerStats) -> bool {
        let error_rate = self.ema_window.error_rate();
        let throughput = self.calculate_throughput(stats);

        error_rate >= self.failure_threshold && throughput >= self.min_throughput_per_second
    }

    fn should_reset(&self, _stats: &BreakerStats) -> bool {
        // Use EMA error rate for recovery decision
        let error_rate = self.ema_window.error_rate();
        error_rate <= self.recovery_threshold
    }
}
