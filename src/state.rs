//! Circuit breaker state machine implementation.

use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

/// Represents the possible states of a circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Circuit is closed and operations are allowed.
    Closed = 0,

    /// Circuit is open and operations are rejected.
    Open = 1,

    /// Circuit is allowing a limited number of operations to test recovery.
    HalfOpen = 2,
}

impl From<u8> for State {
    fn from(value: u8) -> Self {
        match value {
            0 => State::Closed,
            1 => State::Open,
            2 => State::HalfOpen,
            _ => State::Closed, // Default to closed for invalid values
        }
    }
}

/// State transitions representation for the circuit breaker.
pub struct StateManager {
    state: AtomicU8,
    last_transition: parking_lot::Mutex<Instant>,
}

impl StateManager {
    /// Creates a new state manager with the default closed state.
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(State::Closed as u8),
            last_transition: parking_lot::Mutex::new(Instant::now()),
        }
    }

    /// Gets the current state.
    pub fn current(&self) -> State {
        let value = self.state.load(Ordering::Acquire);
        State::from(value)
    }

    /// Gets the time of the last state transition.
    pub fn last_transition_time(&self) -> Instant {
        *self.last_transition.lock()
    }

    /// Duration since the last state transition.
    pub fn time_in_state(&self) -> Duration {
        self.last_transition_time().elapsed()
    }

    /// Attempts to transition from one state to another.
    /// Returns true if the transition succeeded.
    pub fn transition_from_to(&self, from: State, to: State) -> bool {
        let result = self
            .state
            .compare_exchange(from as u8, to as u8, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();

        if result {
            *self.last_transition.lock() = Instant::now();
        }

        result
    }

    /// Attempts to transition to open state from any state.
    pub fn trip_open(&self) -> bool {
        let current = self.current();
        if current == State::Open {
            return false; // Already open
        }

        self.transition_from_to(current, State::Open)
    }

    /// Attempts to transition to half-open state from open state.
    pub fn attempt_half_open(&self) -> bool {
        self.transition_from_to(State::Open, State::HalfOpen)
    }

    /// Attempts to transition to closed state from half-open state.
    pub fn reset_closed(&self) -> bool {
        self.transition_from_to(State::HalfOpen, State::Closed)
    }

    /// Reverts from half-open to open state after failed recovery attempt.
    pub fn revert_to_open(&self) -> bool {
        self.transition_from_to(State::HalfOpen, State::Open)
    }
}
