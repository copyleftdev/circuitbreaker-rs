//! Error types for the circuit breaker library.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Result type for circuit breaker operations.
pub type BreakerResult<T, E> = Result<T, BreakerError<E>>;

/// Error type for circuit breaker operations.
#[derive(Debug)]
pub enum BreakerError<E> {
    /// The circuit is open, calls are not permitted.
    Open,

    /// The underlying operation failed.
    Operation(E),

    /// The circuit breaker encountered an internal error.
    Internal(InternalError),
}

/// Internal errors that can occur within the circuit breaker.
#[derive(Debug)]
pub enum InternalError {
    /// Failed to transition between states.
    StateTransition,

    /// Error in tracking failures.
    FailureTracking,

    /// Error in policy evaluation.
    PolicyEvaluation,

    /// Hook execution failed.
    HookExecution,
}

impl<E> Display for BreakerError<E>
where
    E: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BreakerError::Open => write!(f, "Circuit breaker is open"),
            BreakerError::Operation(e) => write!(f, "Operation error: {}", e),
            BreakerError::Internal(e) => write!(f, "Circuit breaker internal error: {}", e),
        }
    }
}

impl Display for InternalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InternalError::StateTransition => write!(f, "Failed to transition between states"),
            InternalError::FailureTracking => write!(f, "Error in tracking failures"),
            InternalError::PolicyEvaluation => write!(f, "Error in policy evaluation"),
            InternalError::HookExecution => write!(f, "Hook execution failed"),
        }
    }
}

impl<E: Error + 'static> Error for BreakerError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            BreakerError::Open => None,
            BreakerError::Operation(e) => Some(e),
            BreakerError::Internal(_) => None,
        }
    }
}

impl Error for InternalError {}
