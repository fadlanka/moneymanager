//! Spaces out LLM calls. Free tiers fail on requests-per-minute long before
//! requests-per-day, so calls are evenly spaced instead of bursting.

use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::Instant;

pub struct RateLimiter {
    interval: Duration,
    next_slot: Mutex<Option<Instant>>,
}

impl RateLimiter {
    pub fn per_minute(rpm: u32) -> Self {
        Self {
            interval: Duration::from_secs(60) / rpm.max(1),
            next_slot: Mutex::new(None),
        }
    }

    /// Waits until a slot is free, then reserves it.
    pub async fn acquire(&self) {
        let wait_until = {
            let mut next = self.next_slot.lock().await;
            let now = Instant::now();
            let slot = match *next {
                Some(t) if t > now => t,
                _ => now,
            };
            *next = Some(slot + self.interval);
            slot
        };
        tokio::time::sleep_until(wait_until).await;
    }

    /// Push the next slot back, e.g. after a 429.
    pub async fn back_off(&self, d: Duration) {
        let mut next = self.next_slot.lock().await;
        let until = Instant::now() + d;
        if next.is_none_or(|t| t < until) {
            *next = Some(until);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn spaces_calls_evenly() {
        let rl = RateLimiter::per_minute(6); // one every 10s
        let start = Instant::now();
        rl.acquire().await;
        rl.acquire().await;
        rl.acquire().await;
        assert_eq!(start.elapsed(), Duration::from_secs(20));
    }

    #[tokio::test(start_paused = true)]
    async fn back_off_delays_next_call() {
        let rl = RateLimiter::per_minute(60);
        let start = Instant::now();
        rl.back_off(Duration::from_secs(30)).await;
        rl.acquire().await;
        assert_eq!(start.elapsed(), Duration::from_secs(30));
    }
}
