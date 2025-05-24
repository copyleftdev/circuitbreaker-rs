//! Hook registry for circuit breaker events.

use crate::state::State;
use parking_lot::RwLock;
use std::sync::Arc;

type HookFn = Arc<dyn Fn() + Send + Sync + 'static>;

/// A registry for circuit breaker event hooks.
pub struct HookRegistry {
    on_open: RwLock<Option<HookFn>>,
    on_close: RwLock<Option<HookFn>>,
    on_half_open: RwLock<Option<HookFn>>,
    on_success: RwLock<Option<HookFn>>,
    on_failure: RwLock<Option<HookFn>>,
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HookRegistry {
    /// Creates a new empty hook registry.
    pub fn new() -> Self {
        Self {
            on_open: RwLock::new(None),
            on_close: RwLock::new(None),
            on_half_open: RwLock::new(None),
            on_success: RwLock::new(None),
            on_failure: RwLock::new(None),
        }
    }

    /// Sets the hook to call when the circuit breaker opens.
    pub fn set_on_open<F>(&self, f: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self.on_open.write() = Some(Arc::new(f));
    }

    /// Sets the hook to call when the circuit breaker closes.
    pub fn set_on_close<F>(&self, f: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self.on_close.write() = Some(Arc::new(f));
    }

    /// Sets the hook to call when the circuit breaker half-opens.
    pub fn set_on_half_open<F>(&self, f: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self.on_half_open.write() = Some(Arc::new(f));
    }

    /// Sets the hook to call when a call succeeds.
    pub fn set_on_success<F>(&self, f: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self.on_success.write() = Some(Arc::new(f));
    }

    /// Sets the hook to call when a call fails.
    pub fn set_on_failure<F>(&self, f: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self.on_failure.write() = Some(Arc::new(f));
    }

    /// Executes the appropriate hook for a state transition.
    pub fn execute_state_transition_hook(&self, to: State) {
        match to {
            State::Open => {
                if let Some(hook) = self.on_open.read().as_ref() {
                    hook();
                }
            }
            State::Closed => {
                if let Some(hook) = self.on_close.read().as_ref() {
                    hook();
                }
            }
            State::HalfOpen => {
                if let Some(hook) = self.on_half_open.read().as_ref() {
                    hook();
                }
            }
        }
    }

    /// Executes the success hook.
    pub fn execute_success_hook(&self) {
        if let Some(hook) = self.on_success.read().as_ref() {
            hook();
        }
    }

    /// Executes the failure hook.
    pub fn execute_failure_hook(&self) {
        if let Some(hook) = self.on_failure.read().as_ref() {
            hook();
        }
    }
}

#[cfg(feature = "async")]
pub mod async_hooks {
    use crate::state::State;
    use futures::future::BoxFuture;
    use parking_lot::RwLock;
    use std::sync::Arc;

    #[allow(dead_code)]
    type AsyncHookFn = Arc<dyn Fn() -> BoxFuture<'static, ()> + Send + Sync + 'static>;

    /// A registry for asynchronous circuit breaker event hooks.
    #[allow(dead_code)]
    pub struct AsyncHookRegistry {
        on_open: RwLock<Option<AsyncHookFn>>,
        on_close: RwLock<Option<AsyncHookFn>>,
        on_half_open: RwLock<Option<AsyncHookFn>>,
        on_success: RwLock<Option<AsyncHookFn>>,
        on_failure: RwLock<Option<AsyncHookFn>>,
    }

    #[allow(dead_code)]
    #[allow(clippy::await_holding_lock)]
    impl AsyncHookRegistry {
        /// Creates a new empty async hook registry.
        pub fn new() -> Self {
            Self {
                on_open: RwLock::new(None),
                on_close: RwLock::new(None),
                on_half_open: RwLock::new(None),
                on_success: RwLock::new(None),
                on_failure: RwLock::new(None),
            }
        }

        /// Sets the async hook to call when the circuit breaker opens.
        pub fn set_on_open<F, Fut>(&self, f: F)
        where
            F: Fn() -> Fut + Send + Sync + 'static,
            Fut: std::future::Future<Output = ()> + Send + 'static,
        {
            *self.on_open.write() = Some(Arc::new(move || Box::pin(f())));
        }

        /// Sets the async hook to call when the circuit breaker closes.
        pub fn set_on_close<F, Fut>(&self, f: F)
        where
            F: Fn() -> Fut + Send + Sync + 'static,
            Fut: std::future::Future<Output = ()> + Send + 'static,
        {
            *self.on_close.write() = Some(Arc::new(move || Box::pin(f())));
        }

        /// Sets the async hook to call when the circuit breaker half-opens.
        pub fn set_on_half_open<F, Fut>(&self, f: F)
        where
            F: Fn() -> Fut + Send + Sync + 'static,
            Fut: std::future::Future<Output = ()> + Send + 'static,
        {
            *self.on_half_open.write() = Some(Arc::new(move || Box::pin(f())));
        }

        /// Sets the async hook to call when a call succeeds.
        pub fn set_on_success<F, Fut>(&self, f: F)
        where
            F: Fn() -> Fut + Send + Sync + 'static,
            Fut: std::future::Future<Output = ()> + Send + 'static,
        {
            *self.on_success.write() = Some(Arc::new(move || Box::pin(f())));
        }

        /// Sets the async hook to call when a call fails.
        pub fn set_on_failure<F, Fut>(&self, f: F)
        where
            F: Fn() -> Fut + Send + Sync + 'static,
            Fut: std::future::Future<Output = ()> + Send + 'static,
        {
            *self.on_failure.write() = Some(Arc::new(move || Box::pin(f())));
        }

        /// Executes the appropriate async hook for a state transition.
        pub async fn execute_state_transition_hook(&self, to: State) {
            match to {
                State::Open => {
                    if let Some(hook) = self.on_open.read().as_ref() {
                        hook().await;
                    }
                }
                State::Closed => {
                    if let Some(hook) = self.on_close.read().as_ref() {
                        hook().await;
                    }
                }
                State::HalfOpen => {
                    if let Some(hook) = self.on_half_open.read().as_ref() {
                        hook().await;
                    }
                }
            }
        }

        /// Executes the success async hook.
        pub async fn execute_success_hook(&self) {
            if let Some(hook) = self.on_success.read().as_ref() {
                hook().await;
            }
        }

        /// Executes the failure async hook.
        pub async fn execute_failure_hook(&self) {
            if let Some(hook) = self.on_failure.read().as_ref() {
                hook().await;
            }
        }
    }
}
