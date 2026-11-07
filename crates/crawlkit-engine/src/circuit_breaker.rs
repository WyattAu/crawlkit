use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use breaker::{BackoffStrategy, State as CrateState};

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Circuit is closed — requests are allowed.
    Closed,
    /// Circuit is open — requests are blocked.
    Open,
    /// Circuit is testing recovery — requests are allowed.
    HalfOpen,
}

impl From<CrateState> for CircuitState {
    fn from(state: CrateState) -> Self {
        match state {
            CrateState::Closed => CircuitState::Closed,
            CrateState::Open => CircuitState::Open,
            CrateState::HalfOpen => CircuitState::HalfOpen,
        }
    }
}

/// Per-domain circuit breaker for failure isolation.
///
/// Prevents cascade failures by breaking the circuit after consecutive failures.
/// Transitions to Half-Open after cooldown to test recovery.
///
/// The Closed/Open/HalfOpen state machine is delegated to the estate
/// [`breaker`](https://github.com/WyattAu/breaker) crate (consecutive-streak
/// tripping, fixed cooldown, success-threshold half-open recovery); this
/// module keeps the engine's per-domain public API.
///
/// # States
///
/// - **Closed**: Normal operation. Failures increment counter.
/// - **Open**: Requests are blocked. After cooldown, transitions to Half-Open.
/// - **Half-Open**: Limited requests allowed. If they succeed, close circuit.
pub struct CircuitBreaker {
    inner: breaker::CircuitBreaker,
}

/// Configuration for circuit breaker behavior.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit.
    pub failure_threshold: u32,
    /// Number of successes in Half-Open state before closing circuit.
    pub success_threshold: u32,
    /// Duration to wait before transitioning from Open to Half-Open.
    pub cooldown: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            cooldown: Duration::from_secs(60),
        }
    }
}

impl CircuitBreakerConfig {
    /// Map onto the estate crate's config: consecutive-streak tripping only
    /// (sliding-window rate disabled) with a fixed cooldown.
    fn to_crate_config(&self) -> breaker::CircuitBreakerConfig {
        breaker::CircuitBreakerConfig::builder()
            .consecutive_failures(self.failure_threshold)
            .success_threshold(self.success_threshold)
            .failure_rate_threshold(0.0)
            .backoff(BackoffStrategy::Fixed(self.cooldown))
            .build()
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker.
    #[must_use]
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            inner: breaker::CircuitBreaker::new(config.to_crate_config()),
        }
    }

    /// Create with default configuration.
    #[must_use]
    pub fn with_default_config() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Get current state (pure read, no side effects).
    ///
    /// An `Open` circuit whose cooldown has elapsed reads as `HalfOpen`.
    #[must_use]
    pub fn state(&self) -> CircuitState {
        CircuitState::from(self.inner.state())
    }

    /// Check if request is allowed.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        self.inner.is_allowed()
    }

    /// Record a successful request.
    pub fn record_success(&self) {
        self.inner.record_success();
    }

    /// Record a failed request.
    pub fn record_failure(&self) {
        self.inner.record_failure();
    }

    /// Reset circuit breaker to Closed state.
    pub fn reset(&self) {
        self.inner.reset();
    }

    /// Get failure count.
    #[must_use]
    pub fn failure_count(&self) -> u32 {
        self.inner.failure_count()
    }
}

/// Collection of per-domain circuit breakers.
pub struct CircuitBreakerRegistry {
    breakers: dashmap::DashMap<String, Arc<CircuitBreaker>>,
    config: CircuitBreakerConfig,
}

impl CircuitBreakerRegistry {
    /// Create registry with shared configuration.
    #[must_use]
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            breakers: dashmap::DashMap::new(),
            config,
        }
    }

    /// Create with default configuration.
    #[must_use]
    pub fn with_default_config() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Get or create circuit breaker for domain.
    #[must_use]
    pub fn get_or_create(&self, domain: &str) -> CircuitBreakerRef {
        let entry = self
            .breakers
            .entry(domain.to_string())
            .or_insert_with(|| Arc::new(CircuitBreaker::new(self.config.clone())));
        CircuitBreakerRef {
            inner: entry.value().clone(),
        }
    }
}

/// Reference to a domain's circuit breaker.
pub struct CircuitBreakerRef {
    inner: Arc<CircuitBreaker>,
}

impl CircuitBreakerRef {
    /// Check if request is allowed.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        self.inner.is_allowed()
    }

    /// Record success.
    pub fn record_success(&self) {
        self.inner.record_success();
    }

    /// Record failure.
    pub fn record_failure(&self) {
        self.inner.record_failure();
    }

    /// Get current state.
    #[must_use]
    pub fn state(&self) -> CircuitState {
        self.inner.state()
    }

    /// Get failure count.
    #[must_use]
    pub fn failure_count(&self) -> u32 {
        self.inner.failure_count()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_closed_by_default() {
        let cb = CircuitBreaker::with_default_config();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_success_resets_count() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        cb.record_failure();
        cb.record_success(); // Resets failure count
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed); // Not open
    }

    #[test]
    fn test_circuit_breaker_registry() {
        let registry = CircuitBreakerRegistry::with_default_config();
        let cb = registry.get_or_create("example.com");
        assert!(cb.is_allowed());
        cb.record_failure();
        assert_eq!(cb.failure_count(), 1);
    }

    #[test]
    fn test_circuit_breaker_cooldown_gate() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 2,
            cooldown: Duration::from_millis(20),
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_allowed()); // still inside the cooldown

        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_half_open_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 2,
            cooldown: Duration::from_millis(20),
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        std::thread::sleep(Duration::from_millis(50));

        // Success below the threshold keeps HalfOpen.
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // A failure during recovery re-opens immediately.
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_allowed());

        // Recovery after the fresh cooldown closes the circuit.
        std::thread::sleep(Duration::from_millis(50));
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
        assert!(cb.is_allowed());
    }

    #[test]
    fn test_default_config_values() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 3);
        assert_eq!(config.cooldown, Duration::from_secs(60));
    }
}
