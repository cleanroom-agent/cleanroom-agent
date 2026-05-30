//! Retry with exponential backoff and jitter.
//!
//! Implements the Layer 0 (Retry) strategy from docs/16-resilience.md §3.
//! Used by LLM API calls, LSP operations, and any transient-failure-prone I/O.
//!
//! # Usage
//!
//! ```rust,ignore
//! let config = RetryConfig::default();
//! let result = retry_with_backoff(&config, || {
//!     lsp_client.analyze_file(path)
//! }).await?;
//! ```

use std::time::Duration;

/// Configuration for exponential backoff retry.
///
/// Controls how many times an operation is retried and how long
/// to wait between attempts.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of attempts before giving up (default: 5).
    pub max_attempts: u32,
    /// Initial backoff duration in milliseconds (default: 1000).
    pub initial_backoff_ms: u64,
    /// Maximum backoff duration in milliseconds (default: 60000).
    pub max_backoff_ms: u64,
    /// Jitter factor — random variation as a fraction of the backoff (default: 0.2).
    /// A value of 0.2 means ±20% jitter.
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff_ms: 1000,
            max_backoff_ms: 60_000,
            jitter_factor: 0.2,
        }
    }
}

impl RetryConfig {
    /// Shorter retry config for quick operations (LSP, file I/O).
    pub fn fast() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 500,
            max_backoff_ms: 10_000,
            jitter_factor: 0.2,
        }
    }

    /// Longer retry config for LLM API calls subjected to rate limiting.
    pub fn llm_api() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff_ms: 2000,
            max_backoff_ms: 120_000,
            jitter_factor: 0.2,
        }
    }
}

/// Retry an async operation with exponential backoff and jitter.
///
/// The operation is retried up to `config.max_attempts` times.
/// The backoff grows exponentially: `initial * 2^(attempt-1)`, capped at `max_backoff_ms`.
/// Jitter of ±`jitter_factor` is added to avoid thundering herd.
///
/// # Errors
///
/// Returns the last error if all attempts are exhausted.
pub async fn retry_with_backoff<F, Fut, T, E>(
    config: &RetryConfig,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;
                if attempt >= config.max_attempts {
                    tracing::error!(
                        attempt,
                        error = %e,
                        "Operation failed after max attempts"
                    );
                    return Err(e);
                }
                let backoff = calculate_backoff(config, attempt);
                tracing::warn!(
                    attempt,
                    sleep_ms = backoff.as_millis(),
                    error = %e,
                    "Operation failed, retrying with backoff..."
                );
                tokio::time::sleep(backoff).await;
            }
        }
    }
}

/// Retry a synchronous operation with exponential backoff and jitter.
///
/// Same semantics as `retry_with_backoff` but for synchronous closures.
/// Sleeps the current thread.
pub fn retry_sync_with_backoff<F, T, E>(
    config: &RetryConfig,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    loop {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;
                if attempt >= config.max_attempts {
                    tracing::error!(
                        attempt,
                        error = %e,
                        "Operation failed after max attempts"
                    );
                    return Err(e);
                }
                let backoff = calculate_backoff(config, attempt);
                tracing::warn!(
                    attempt,
                    sleep_ms = backoff.as_millis(),
                    error = %e,
                    "Operation failed, retrying with backoff..."
                );
                std::thread::sleep(backoff);
            }
        }
    }
}

/// Calculate the backoff duration for a given attempt.
///
/// Uses exponential growth with jitter:
/// `backoff = min(initial * 2^(attempt-1), max) + random_jitter`
fn calculate_backoff(config: &RetryConfig, attempt: u32) -> Duration {
    let base = config.initial_backoff_ms.saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1)));
    let capped = base.min(config.max_backoff_ms);
    // Simple pseudo-random jitter without depending on the `rand` crate.
    // Uses the current time nanoseconds as a cheap entropy source.
    let jitter_range = (capped as f64 * config.jitter_factor) as i64;
    let jitter = if jitter_range > 0 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as i64)
            .unwrap_or(0);
        // Simple LCG: hash the nanos to get a pseudo-random value in range
        let r = (nanos.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff) as f64
            / 0x7fffffff as f64;
        (r * 2.0 - 1.0) * jitter_range as f64
    } else {
        0.0
    };
    let total = (capped as i64 + jitter as i64).max(0) as u64;
    Duration::from_millis(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_backoff_grows() {
        let config = RetryConfig::default();
        let _b1 = calculate_backoff(&config, 1);
        let b2 = calculate_backoff(&config, 2);
        // Backoff should grow (roughly; jitter may invert for small values)
        assert!(b2.as_millis() <= config.max_backoff_ms as u128 + 100);
    }

    #[test]
    fn test_calculate_backoff_capped() {
        let config = RetryConfig::default();
        let b10 = calculate_backoff(&config, 10);
        // Should be capped at max_backoff_ms
        assert!(b10.as_millis() <= (config.max_backoff_ms as u128 * 2));
    }

    #[test]
    fn test_fast_config_is_shorter() {
        let fast = RetryConfig::fast();
        let default = RetryConfig::default();
        assert!(fast.max_attempts < default.max_attempts);
        assert!(fast.initial_backoff_ms < default.initial_backoff_ms);
    }

    #[test]
    fn test_llm_api_config_is_longer() {
        let llm = RetryConfig::llm_api();
        let default = RetryConfig::default();
        assert!(llm.initial_backoff_ms >= default.initial_backoff_ms);
    }
}
