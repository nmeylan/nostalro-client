use std::time::{Duration, Instant};

pub const BGM_RETRY_INTERVAL: Duration = Duration::from_secs(10);

/// A BGM track that could not be loaded yet. Unlike a missing wav — a data
/// problem that will not fix itself — a missing track is often a mount or a
/// download still in flight, so it is re-attempted instead of blacklisted.
#[derive(Default)]
pub struct BgmRetry {
    pending: Option<(String, Instant)>,
}

impl BgmRetry {
    pub fn schedule(&mut self, key: &str, now: Instant) {
        self.pending = Some((key.to_string(), now + BGM_RETRY_INTERVAL));
    }

    pub fn take_due(&mut self, now: Instant) -> Option<String> {
        match &self.pending {
            Some((_, at)) if now >= *at => self.pending.take().map(|(key, _)| key),
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.pending = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_waits_for_the_full_interval_and_fires_once() {
        let now = Instant::now();
        let mut retry = BgmRetry::default();
        retry.schedule("01.mp3", now);

        assert_eq!(retry.take_due(now), None);
        assert_eq!(retry.take_due(now + BGM_RETRY_INTERVAL / 2), None);
        assert_eq!(
            retry.take_due(now + BGM_RETRY_INTERVAL),
            Some("01.mp3".to_string())
        );
        assert_eq!(retry.take_due(now + BGM_RETRY_INTERVAL * 3), None);
    }

    #[test]
    fn clear_cancels_a_pending_retry() {
        let now = Instant::now();
        let mut retry = BgmRetry::default();
        retry.schedule("01.mp3", now);
        retry.clear();
        assert_eq!(retry.take_due(now + BGM_RETRY_INTERVAL * 2), None);
    }
}
