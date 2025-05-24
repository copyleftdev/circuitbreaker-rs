//! Failure tracking and metrics for circuit breaker.

use parking_lot::Mutex;
use smallvec::SmallVec;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Trait for metrics sinks that can receive circuit breaker events.
pub trait MetricSink: Send + Sync + 'static {
    /// Records a state transition event.
    fn record_state_transition(&self, from: &str, to: &str);

    /// Records an error rate change.
    fn record_error_rate(&self, rate: f64);

    /// Records a probe attempt.
    fn record_probe_attempt(&self, success: bool);

    /// Records a call result.
    fn record_call(&self, success: bool, duration: Duration);
}

/// A null metrics sink that discards all events.
pub struct NullMetricSink;

impl MetricSink for NullMetricSink {
    fn record_state_transition(&self, _from: &str, _to: &str) {}
    fn record_error_rate(&self, _rate: f64) {}
    fn record_probe_attempt(&self, _success: bool) {}
    fn record_call(&self, _success: bool, _duration: Duration) {}
}

/// Statistics for the circuit breaker.
#[derive(Debug)]
pub struct BreakerStats {
    success_count: AtomicU64,
    failure_count: AtomicU64,
    consecutive_failures: AtomicU64,
    consecutive_successes: AtomicU64,
    last_failure_time: Mutex<Option<Instant>>,
    last_success_time: Mutex<Option<Instant>>,
    total_calls: AtomicU64,
}

impl Default for BreakerStats {
    fn default() -> Self {
        Self::new()
    }
}

impl BreakerStats {
    /// Creates a new empty stats tracker.
    pub fn new() -> Self {
        Self {
            success_count: AtomicU64::new(0),
            failure_count: AtomicU64::new(0),
            consecutive_failures: AtomicU64::new(0),
            consecutive_successes: AtomicU64::new(0),
            last_failure_time: Mutex::new(None),
            last_success_time: Mutex::new(None),
            total_calls: AtomicU64::new(0),
        }
    }

    /// Gets the current success count.
    pub fn get_success_count(&self) -> u64 {
        self.success_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gets the current failure count.
    pub fn get_failure_count(&self) -> u64 {
        self.failure_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gets the total call count.
    pub fn get_total_calls(&self) -> u64 {
        self.total_calls.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gets the last failure time.
    pub fn get_last_failure_time(&self) -> Option<Instant> {
        *self.last_failure_time.lock()
    }

    /// Records a successful call.
    pub fn record_success(&self) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
        self.consecutive_successes.fetch_add(1, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        *self.last_success_time.lock() = Some(Instant::now());
    }

    /// Records a failed call.
    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
        self.consecutive_successes.store(0, Ordering::Relaxed);
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        *self.last_failure_time.lock() = Some(Instant::now());
    }

    /// Gets the current error rate.
    pub fn error_rate(&self) -> f64 {
        let failures = self.failure_count.load(Ordering::Relaxed);
        let total = self.total_calls.load(Ordering::Relaxed);

        if total == 0 {
            return 0.0;
        }

        failures as f64 / total as f64
    }

    /// Gets the number of consecutive failures.
    pub fn consecutive_failures(&self) -> u64 {
        self.consecutive_failures.load(Ordering::Relaxed)
    }

    /// Gets the number of consecutive successes.
    pub fn consecutive_successes(&self) -> u64 {
        self.consecutive_successes.load(Ordering::Relaxed)
    }

    /// Resets all statistics.
    pub fn reset(&self) {
        self.success_count.store(0, Ordering::Relaxed);
        self.failure_count.store(0, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);
        self.consecutive_successes.store(0, Ordering::Relaxed);
        self.total_calls.store(0, Ordering::Relaxed);
        *self.last_failure_time.lock() = None;
        *self.last_success_time.lock() = None;
    }
}

/// A time window for tracking failures with fixed buckets.
pub struct FixedWindow {
    buckets: Mutex<SmallVec<[(Instant, u64, u64); 16]>>, // (timestamp, successes, failures)
    window_size: Duration,
    bucket_size: Duration,
}

impl FixedWindow {
    /// Creates a new fixed window tracker.
    pub fn new(window_size: Duration, bucket_count: usize) -> Self {
        let bucket_size = window_size / bucket_count as u32;
        Self {
            buckets: Mutex::new(SmallVec::new()),
            window_size,
            bucket_size,
        }
    }

    /// Records a successful call.
    pub fn record_success(&self) {
        let mut buckets = self.buckets.lock();
        self.clean_old_buckets(&mut buckets);

        let now = Instant::now();
        if let Some(bucket) = buckets.last_mut() {
            if now.duration_since(bucket.0) < self.bucket_size {
                bucket.1 += 1;
                return;
            }
        }

        buckets.push((now, 1, 0));
    }

    /// Records a failed call.
    pub fn record_failure(&self) {
        let mut buckets = self.buckets.lock();
        self.clean_old_buckets(&mut buckets);

        let now = Instant::now();
        if let Some(bucket) = buckets.last_mut() {
            if now.duration_since(bucket.0) < self.bucket_size {
                bucket.2 += 1;
                return;
            }
        }

        buckets.push((now, 0, 1));
    }

    /// Gets the current error rate in the window.
    pub fn error_rate(&self) -> f64 {
        let mut buckets = self.buckets.lock();
        self.clean_old_buckets(&mut buckets);

        let mut total_success = 0;
        let mut total_failure = 0;

        for (_, successes, failures) in buckets.iter() {
            total_success += successes;
            total_failure += failures;
        }

        let total = total_success + total_failure;
        if total == 0 {
            return 0.0;
        }

        total_failure as f64 / total as f64
    }

    fn clean_old_buckets(&self, buckets: &mut SmallVec<[(Instant, u64, u64); 16]>) {
        let now = Instant::now();
        let cutoff = now - self.window_size;

        while let Some(bucket) = buckets.first() {
            if bucket.0 < cutoff {
                buckets.remove(0);
            } else {
                break;
            }
        }
    }
}

/// A time window for tracking failures with exponential moving average.
pub struct EMAWindow {
    error_rate: AtomicU64, // Stored as bits of f64
    alpha: f64,
    calls_required: u64,
    call_count: AtomicU64,
}

impl EMAWindow {
    /// Creates a new EMA window tracker.
    pub fn new(alpha: f64, calls_required: u64) -> Self {
        Self {
            error_rate: AtomicU64::new(0),
            alpha,
            calls_required,
            call_count: AtomicU64::new(0),
        }
    }

    /// Records a successful call.
    pub fn record_success(&self) {
        self.call_count.fetch_add(1, Ordering::Relaxed);

        if self.call_count.load(Ordering::Relaxed) < self.calls_required {
            return;
        }

        let current = f64::from_bits(self.error_rate.load(Ordering::Relaxed));
        let new = current * (1.0 - self.alpha);
        self.error_rate.store(new.to_bits(), Ordering::Relaxed);
    }

    /// Records a failed call.
    pub fn record_failure(&self) {
        self.call_count.fetch_add(1, Ordering::Relaxed);

        if self.call_count.load(Ordering::Relaxed) < self.calls_required {
            return;
        }

        let current = f64::from_bits(self.error_rate.load(Ordering::Relaxed));
        let new = current * (1.0 - self.alpha) + self.alpha;
        self.error_rate.store(new.to_bits(), Ordering::Relaxed);
    }

    /// Gets the current EMA error rate.
    pub fn error_rate(&self) -> f64 {
        if self.call_count.load(Ordering::Relaxed) < self.calls_required {
            return 0.0;
        }

        f64::from_bits(self.error_rate.load(Ordering::Relaxed))
    }
}
