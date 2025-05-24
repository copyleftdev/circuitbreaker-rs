//! Core circuit breaker implementation.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::error::{BreakerError, BreakerResult};
use crate::hook::HookRegistry;
use crate::metrics::{BreakerStats, MetricSink};
use crate::policy::BreakerPolicy;
use crate::state::{State, StateManager};

/// Inner state of the circuit breaker, shared between instances.
struct BreakerInner<P>
where
    P: BreakerPolicy,
{
    state_manager: StateManager,
    policy: P,
    stats: BreakerStats,
    cooldown_duration: Duration,
    probes_allowed: AtomicU32,
    probe_interval: u32,
    last_probe_time: parking_lot::Mutex<Instant>,
    metric_sink: Arc<dyn MetricSink>,
    hooks: Arc<HookRegistry>,
}

/// A circuit breaker that can wrap function calls to prevent cascading failures.
pub struct CircuitBreaker<P, E>
where
    P: BreakerPolicy,
    E: std::error::Error + 'static,
{
    inner: Arc<BreakerInner<P>>,
    _error_type: std::marker::PhantomData<E>,
}

impl<P, E> CircuitBreaker<P, E>
where
    P: BreakerPolicy,
    E: std::error::Error + 'static,
{
    /// Creates a new circuit breaker with the specified policy and settings.
    pub fn new(
        policy: P,
        cooldown_duration: Duration,
        probe_interval: u32,
        metric_sink: Arc<dyn MetricSink>,
        hooks: Arc<HookRegistry>,
    ) -> Self {
        let inner = BreakerInner {
            state_manager: StateManager::new(),
            policy,
            stats: BreakerStats::new(),
            cooldown_duration,
            probes_allowed: AtomicU32::new(0),
            probe_interval,
            last_probe_time: parking_lot::Mutex::new(Instant::now()),
            metric_sink,
            hooks,
        };

        Self {
            inner: Arc::new(inner),
            _error_type: std::marker::PhantomData,
        }
    }

    /// Creates a new builder for customizing a circuit breaker.
    pub fn builder() -> crate::config::BreakerBuilder<crate::policy::DefaultPolicy, E> {
        crate::config::BreakerBuilder::new()
    }

    /// Gets the current state of the circuit breaker.
    pub fn current_state(&self) -> State {
        self.inner.state_manager.current()
    }

    /// Gets the current error rate of the circuit breaker.
    pub fn error_rate(&self) -> f64 {
        self.inner.stats.error_rate()
    }

    /// Executes a function wrapped by the circuit breaker.
    pub fn call<F, T>(&self, f: F) -> BreakerResult<T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        self.pre_call()?;

        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        self.post_call(&result, duration);

        result.map_err(BreakerError::Operation)
    }

    /// Checks if a call is allowed based on the current state.
    fn pre_call(&self) -> Result<(), BreakerError<E>> {
        match self.inner.state_manager.current() {
            State::Closed => Ok(()),
            State::Open => {
                // Check if cooldown period has elapsed
                if self.inner.state_manager.time_in_state() >= self.inner.cooldown_duration {
                    // Attempt to transition to half-open
                    if self.inner.state_manager.attempt_half_open() {
                        // Reset probe counter
                        self.inner
                            .probes_allowed
                            .store(self.inner.probe_interval, Ordering::Relaxed);
                        *self.inner.last_probe_time.lock() = Instant::now();

                        // Execute hook outside the lock path
                        self.inner
                            .hooks
                            .execute_state_transition_hook(State::HalfOpen);

                        // Record metric
                        self.inner
                            .metric_sink
                            .record_state_transition("open", "half-open");

                        return Ok(());
                    }
                }

                Err(BreakerError::Open)
            }
            State::HalfOpen => {
                // Check if we have probes left
                let probes = self.inner.probes_allowed.load(Ordering::Relaxed);
                if probes > 0 {
                    // Decrement probe counter
                    self.inner.probes_allowed.fetch_sub(1, Ordering::Relaxed);

                    // Record metric
                    self.inner.metric_sink.record_probe_attempt(true);

                    Ok(())
                } else {
                    // Record metric
                    self.inner.metric_sink.record_probe_attempt(false);

                    Err(BreakerError::Open)
                }
            }
        }
    }

    /// Processes the result of a call to update stats and potentially change state.
    fn post_call<T>(&self, result: &Result<T, E>, duration: Duration) {
        let success = result.is_ok();
        let current_state = self.inner.state_manager.current();

        // Record metrics
        self.inner.metric_sink.record_call(success, duration);

        if success {
            self.inner.stats.record_success();
            self.inner.hooks.execute_success_hook();

            // If in half-open state and should reset to closed
            if current_state == State::HalfOpen
                && self.inner.policy.should_reset(&self.inner.stats)
                && self.inner.state_manager.reset_closed()
            {
                // Reset stats
                self.inner.stats.reset();

                // Execute hook outside the lock path
                self.inner
                    .hooks
                    .execute_state_transition_hook(State::Closed);

                // Record metric
                self.inner
                    .metric_sink
                    .record_state_transition("half-open", "closed");
            }
        } else {
            self.inner.stats.record_failure();
            self.inner.hooks.execute_failure_hook();

            // If in half-open state, revert to open
            if current_state == State::HalfOpen {
                if self.inner.state_manager.revert_to_open() {
                    // Execute hook outside the lock path
                    self.inner.hooks.execute_state_transition_hook(State::Open);

                    // Record metric
                    self.inner
                        .metric_sink
                        .record_state_transition("half-open", "open");
                }
            } else if current_state == State::Closed
                && self.inner.policy.should_trip(&self.inner.stats)
            {
                // If in closed state and should trip
                if self.inner.state_manager.trip_open() {
                    // Execute hook outside the lock path
                    self.inner.hooks.execute_state_transition_hook(State::Open);

                    // Record metric
                    self.inner
                        .metric_sink
                        .record_state_transition("closed", "open");
                    self.inner
                        .metric_sink
                        .record_error_rate(self.inner.stats.error_rate());
                }
            }
        }
    }

    /// Forces the circuit breaker to the open state.
    pub fn force_open(&self) -> bool {
        let current = self.inner.state_manager.current();
        if current == State::Open {
            return false;
        }

        let result = self.inner.state_manager.trip_open();
        if result {
            // Execute hook outside the lock path
            self.inner.hooks.execute_state_transition_hook(State::Open);

            // Record metric
            self.inner.metric_sink.record_state_transition(
                match current {
                    State::Closed => "closed",
                    State::HalfOpen => "half-open",
                    State::Open => "open", // Shouldn't happen
                },
                "open",
            );
        }

        result
    }

    /// Forces the circuit breaker to the closed state.
    pub fn force_closed(&self) -> bool {
        let current = self.inner.state_manager.current();
        if current == State::Closed {
            return false;
        }

        let result = match current {
            State::Open => self
                .inner
                .state_manager
                .transition_from_to(State::Open, State::Closed),
            State::HalfOpen => self.inner.state_manager.reset_closed(),
            State::Closed => false, // Already closed
        };

        if result {
            // Reset stats
            self.inner.stats.reset();

            // Execute hook outside the lock path
            self.inner
                .hooks
                .execute_state_transition_hook(State::Closed);

            // Record metric
            self.inner.metric_sink.record_state_transition(
                match current {
                    State::Open => "open",
                    State::HalfOpen => "half-open",
                    State::Closed => "closed", // Shouldn't happen
                },
                "closed",
            );
        }

        result
    }

    /// Resets the circuit breaker's statistics.
    pub fn reset_stats(&self) {
        self.inner.stats.reset();
    }
}

// Allow cloning of circuit breakers - cheap because inner state is Arc'd
impl<P, E> Clone for CircuitBreaker<P, E>
where
    P: BreakerPolicy,
    E: std::error::Error + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            _error_type: std::marker::PhantomData,
        }
    }
}

// Implement Async support when the feature is enabled
#[cfg(feature = "async")]
impl<P, E> CircuitBreaker<P, E>
where
    P: BreakerPolicy,
    E: std::error::Error + 'static,
{
    /// Executes an async function wrapped by the circuit breaker.
    pub async fn call_async<F, Fut, T>(&self, f: F) -> BreakerResult<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        self.pre_call()?;

        let start = Instant::now();
        let result = f().await;
        let duration = start.elapsed();

        self.post_call(&result, duration);

        result.map_err(BreakerError::Operation)
    }
}
