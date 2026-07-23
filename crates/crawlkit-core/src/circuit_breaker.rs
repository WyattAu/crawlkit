use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Per-domain circuit breaker for failure isolation.
///
/// Prevents cascade failures by breaking the circuit after consecutive failures.
/// Transitions to Half-Open after cooldown to test recovery.
pub struct CircuitBreaker {
    state: AtomicU8,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: AtomicU64,
    config: CircuitBreakerConfig,
}

/// Configuration for circuit breaker behavior.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit.
    pub failure_threshold: u32,
    /// Number of successes in Half-Open before closing.
    pub success_threshold: u32,
    /// Cooldown before transitioning from Open to Half-Open.
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

impl CircuitBreaker {
    /// Create a new circuit breaker.
    #[must_use]
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            config,
        }
    }

    /// Create with default configuration.
    #[must_use]
    pub fn with_default_config() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Get current state.
    #[must_use]
    pub fn state(&self) -> CircuitState {
        let state = self.state.load(Ordering::Acquire);
        match state {
            0 => CircuitState::Closed,
            1 => {
                // Check if cooldown expired
                let last_failure = self.last_failure_time.load(Ordering::Acquire);
                let now = now_millis();
                if now.saturating_sub(last_failure) >= self.config.cooldown.as_millis() as u64 {
                    self.state
                        .store(CircuitState::HalfOpen as u8, Ordering::Release);
                    CircuitState::HalfOpen
                } else {
                    CircuitState::Open
                }
            }
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }

    /// Check if request is allowed.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        match self.state() {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true,
            CircuitState::Open => false,
        }
    }

    /// Record a successful request.
    pub fn record_success(&self) {
        match self.state() {
            CircuitState::Closed => {
                self.failure_count.store(0, Ordering::Release);
                self.success_count.store(0, Ordering::Release);
            }
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::AcqRel) + 1;
                if successes >= self.config.success_threshold {
                    self.state
                        .store(CircuitState::Closed as u8, Ordering::Release);
                    self.failure_count.store(0, Ordering::Release);
                    self.success_count.store(0, Ordering::Release);
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failed request.
    pub fn record_failure(&self) {
        let now = now_millis();
        self.last_failure_time.store(now, Ordering::Release);

        match self.state() {
            CircuitState::Closed => {
                let failures = self.failure_count.fetch_add(1, Ordering::AcqRel) + 1;
                if failures >= self.config.failure_threshold {
                    self.state
                        .store(CircuitState::Open as u8, Ordering::Release);
                }
            }
            CircuitState::HalfOpen => {
                self.state
                    .store(CircuitState::Open as u8, Ordering::Release);
                self.success_count.store(0, Ordering::Release);
            }
            CircuitState::Open => {}
        }
    }

    /// Reset circuit breaker to Closed state.
    pub fn reset(&self) {
        self.state
            .store(CircuitState::Closed as u8, Ordering::Release);
        self.failure_count.store(0, Ordering::Release);
        self.success_count.store(0, Ordering::Release);
    }

    /// Get failure count.
    #[must_use]
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Acquire)
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

/// Get current time in milliseconds since epoch.
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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
}
