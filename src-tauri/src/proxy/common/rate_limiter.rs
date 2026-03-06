// Rate Limiter
// 确保 API 调用间隔 ≥ 500ms

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant as StdInstant;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};
use dashmap::DashMap;

pub struct RateLimiter {
    min_interval: Duration,
    last_call: Arc<Mutex<Option<Instant>>>,
}

impl RateLimiter {
    pub fn new(min_interval_ms: u64) -> Self {
        Self {
            min_interval: Duration::from_millis(min_interval_ms),
            last_call: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn wait(&self) {
        let mut last = self.last_call.lock().await;
        if let Some(last_time) = *last {
            let elapsed = last_time.elapsed();
            if elapsed < self.min_interval {
                sleep(self.min_interval - elapsed).await;
            }
        }
        *last = Some(Instant::now());
    }
}

// ============================================================================
// Per-Account RPM Tracker (Proactive rate limiting)
// Sliding 60s window to cap requests per account before hitting 429s
// ============================================================================

const RPM_WINDOW_SECS: u64 = 60;

/// Tracks per-account request timestamps in a sliding 60s window.
/// Thread-safe via DashMap — no external lock needed.
pub struct AccountRequestTracker {
    requests: DashMap<String, VecDeque<StdInstant>>,
}

impl AccountRequestTracker {
    pub fn new() -> Self {
        Self {
            requests: DashMap::new(),
        }
    }

    /// Record a request for the given account. Prunes entries older than 60s.
    pub fn record_request(&self, account_id: &str) {
        let now = StdInstant::now();
        let cutoff = now - std::time::Duration::from_secs(RPM_WINDOW_SECS);

        let mut entry = self.requests.entry(account_id.to_string()).or_insert_with(VecDeque::new);
        // Prune expired timestamps
        while entry.front().map_or(false, |t| *t < cutoff) {
            entry.pop_front();
        }
        entry.push_back(now);
    }

    /// Get the current RPM (requests in the last 60s) for an account.
    pub fn get_rpm(&self, account_id: &str) -> u32 {
        let now = StdInstant::now();
        let cutoff = now - std::time::Duration::from_secs(RPM_WINDOW_SECS);

        if let Some(mut entry) = self.requests.get_mut(account_id) {
            // Prune expired timestamps
            while entry.front().map_or(false, |t| *t < cutoff) {
                entry.pop_front();
            }
            entry.len() as u32
        } else {
            0
        }
    }

    /// Check if an account has exceeded the given RPM limit.
    pub fn is_over_limit(&self, account_id: &str, max_rpm: u32) -> bool {
        self.get_rpm(account_id) >= max_rpm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(500);
        let start = Instant::now();

        limiter.wait().await;
        let elapsed1 = start.elapsed().as_millis();
        assert!(elapsed1 < 50);

        limiter.wait().await;
        let elapsed2 = start.elapsed().as_millis();
        assert!(elapsed2 >= 500 && elapsed2 < 600);
    }

    #[test]
    fn test_account_request_tracker_basic() {
        let tracker = AccountRequestTracker::new();

        assert_eq!(tracker.get_rpm("acc1"), 0);
        assert!(!tracker.is_over_limit("acc1", 10));

        for _ in 0..5 {
            tracker.record_request("acc1");
        }
        assert_eq!(tracker.get_rpm("acc1"), 5);
        assert!(!tracker.is_over_limit("acc1", 10));

        for _ in 0..5 {
            tracker.record_request("acc1");
        }
        assert_eq!(tracker.get_rpm("acc1"), 10);
        assert!(tracker.is_over_limit("acc1", 10));
    }

    #[test]
    fn test_account_request_tracker_isolation() {
        let tracker = AccountRequestTracker::new();

        for _ in 0..5 {
            tracker.record_request("acc1");
        }
        for _ in 0..3 {
            tracker.record_request("acc2");
        }

        assert_eq!(tracker.get_rpm("acc1"), 5);
        assert_eq!(tracker.get_rpm("acc2"), 3);
        assert_eq!(tracker.get_rpm("acc3"), 0);
    }
}
