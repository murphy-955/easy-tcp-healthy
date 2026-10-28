use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MAX_REQUESTS: usize = 10;
const WINDOW: Duration = Duration::from_secs(60);

struct RateWindow {
    started_at: Instant,
    count: usize,
}

pub(crate) struct RateLimiter {
    clients: Mutex<HashMap<IpAddr, RateWindow>>,
}

impl RateLimiter {
    pub(crate) fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn check(&self, ip: IpAddr) -> Result<(), u64> {
        let now = Instant::now();
        let mut clients = self.clients.lock().unwrap();
        clients.retain(|_, window| now.duration_since(window.started_at) < WINDOW);

        let window = clients.entry(ip).or_insert(RateWindow {
            started_at: now,
            count: 0,
        });
        if window.count >= MAX_REQUESTS {
            let remaining = WINDOW.saturating_sub(now.duration_since(window.started_at));
            return Err(remaining.as_secs().max(1));
        }

        window.count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_ten_requests_and_limits_the_eleventh() {
        let limiter = RateLimiter::new();
        let ip = IpAddr::from([127, 0, 0, 1]);

        for _ in 0..10 {
            assert!(limiter.check(ip).is_ok());
        }
        assert!(limiter.check(ip).is_err());
    }
}
