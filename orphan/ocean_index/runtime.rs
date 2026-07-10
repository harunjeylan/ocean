use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl RetryPolicy {
    pub fn new(max_retries: u32, initial_backoff_ms: u64, max_backoff_ms: u64) -> Self {
        Self { max_retries, initial_backoff_ms, max_backoff_ms }
    }

    pub fn next_delay(&self, retry_count: u32) -> Duration {
        let ms = self.initial_backoff_ms.saturating_mul(2u64.saturating_pow(retry_count));
        Duration::from_millis(ms.min(self.max_backoff_ms))
    }

    pub fn is_transient(&self, error: &dyn std::error::Error) -> bool {
        let msg = error.to_string().to_lowercase();
        let transient_keywords = [
            "timeout", "timed out", "rate limit", "too many requests",
            "conflict", "503", "429", "connection refused",
            "connection reset", "temporarily unavailable",
            "service unavailable", "throttl",
        ];
        transient_keywords.iter().any(|kw| msg.contains(kw))
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(3, 100, 30_000)
    }
}
