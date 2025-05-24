//! Configuration for circuit breakers.

use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use crate::breaker::CircuitBreaker;
use crate::hook::HookRegistry;
use crate::metrics::{MetricSink, NullMetricSink};
use crate::policy::{BreakerPolicy, DefaultPolicy};

/// Builder for creating circuit breakers with custom configurations.
pub struct BreakerBuilder<P, E>
where
    P: BreakerPolicy,
    E: std::error::Error + 'static,
{
    failure_threshold: f64,
    min_throughput: u64,
    cooldown_duration: Duration,
    probe_interval: u32,
    consecutive_failures_threshold: u64,
    consecutive_successes_threshold: u64,
    policy: Option<P>,
    metric_sink: Arc<dyn MetricSink>,
    hook_registry: Arc<HookRegistry>,
    _error_type: PhantomData<E>,
}

impl<E> Default for BreakerBuilder<DefaultPolicy, E>
where
    E: std::error::Error + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<E> BreakerBuilder<DefaultPolicy, E>
where
    E: std::error::Error + 'static,
{
    /// Creates a new builder with default settings.
    pub fn new() -> Self {
        Self {
            failure_threshold: 0.5,
            min_throughput: 10,
            cooldown_duration: Duration::from_secs(30),
            probe_interval: 5,
            consecutive_failures_threshold: 5,
            consecutive_successes_threshold: 3,
            policy: None,
            metric_sink: Arc::new(NullMetricSink),
            hook_registry: Arc::new(HookRegistry::new()),
            _error_type: PhantomData,
        }
    }
}

impl<P, E> BreakerBuilder<P, E>
where
    P: BreakerPolicy,
    E: std::error::Error + 'static,
{
    /// Sets the failure rate threshold that will trip the circuit.
    pub fn failure_threshold(mut self, threshold: f64) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Sets the minimum number of calls required before considering the error rate.
    pub fn min_throughput(mut self, min_throughput: u64) -> Self {
        self.min_throughput = min_throughput;
        self
    }

    /// Sets the cooldown duration before the circuit transitions from open to half-open.
    pub fn cooldown(mut self, duration: Duration) -> Self {
        self.cooldown_duration = duration;
        self
    }

    /// Sets the number of probes to allow in half-open state.
    pub fn probe_interval(mut self, interval: u32) -> Self {
        self.probe_interval = interval;
        self
    }

    /// Sets the number of consecutive failures required to trip the circuit.
    pub fn consecutive_failures(mut self, count: u64) -> Self {
        self.consecutive_failures_threshold = count;
        self
    }

    /// Sets the number of consecutive successes required to reset the circuit.
    pub fn consecutive_successes(mut self, count: u64) -> Self {
        self.consecutive_successes_threshold = count;
        self
    }

    /// Sets a custom policy for the circuit breaker.
    pub fn policy(mut self, policy: P) -> Self {
        self.policy = Some(policy);
        self
    }

    /// Sets a metric sink for the circuit breaker.
    pub fn metric_sink<M: MetricSink>(mut self, sink: M) -> Self {
        self.metric_sink = Arc::new(sink);
        self
    }

    /// Sets a hook registry for the circuit breaker.
    pub fn hooks(mut self, hooks: HookRegistry) -> Self {
        self.hook_registry = Arc::new(hooks);
        self
    }

    /// Changes the error type for the builder.
    pub fn with_error_type<NewE: std::error::Error + 'static>(self) -> BreakerBuilder<P, NewE> {
        BreakerBuilder {
            failure_threshold: self.failure_threshold,
            min_throughput: self.min_throughput,
            cooldown_duration: self.cooldown_duration,
            probe_interval: self.probe_interval,
            consecutive_failures_threshold: self.consecutive_failures_threshold,
            consecutive_successes_threshold: self.consecutive_successes_threshold,
            policy: self.policy,
            metric_sink: self.metric_sink,
            hook_registry: self.hook_registry,
            _error_type: PhantomData,
        }
    }

    /// Builds a new circuit breaker with the configured settings.
    /// This method is available only for non-DefaultPolicy implementations.
    pub fn build_with_policy(self) -> CircuitBreaker<P, E> {
        match self.policy {
            Some(policy) => CircuitBreaker::new(
                policy,
                self.cooldown_duration,
                self.probe_interval,
                self.metric_sink,
                self.hook_registry,
            ),
            None => panic!("Policy must be provided when not using DefaultPolicy"),
        }
    }
}

impl<E> BreakerBuilder<DefaultPolicy, E>
where
    E: std::error::Error + 'static,
{
    /// Builds a circuit breaker with the default policy.
    pub fn build(self) -> CircuitBreaker<DefaultPolicy, E> {
        let policy = DefaultPolicy::new(
            self.failure_threshold,
            self.min_throughput,
            self.consecutive_failures_threshold,
            self.consecutive_successes_threshold,
        );

        CircuitBreaker::new(
            policy,
            self.cooldown_duration,
            self.probe_interval,
            self.metric_sink,
            self.hook_registry,
        )
    }
}
