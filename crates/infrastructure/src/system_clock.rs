use std::time::{SystemTime, UNIX_EPOCH};

use application::Clock;

/// Horloge système réelle.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_is_after_2020() {
        assert!(SystemClock.now_unix_secs() > 1_577_836_800);
    }
}
