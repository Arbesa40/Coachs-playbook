// Hybrid logical clock encoded as "millis-counter-device" so string order is time order.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Timestamp {
    pub millis: u64,
    pub counter: u32,
}

impl Timestamp {
    pub fn encode(&self, device: &str) -> String {
        format!("{:013}-{:06}-{}", self.millis, self.counter, device)
    }

    /// Returns the timestamp and the device id embedded in an encoded HLC string.
    pub fn parse(s: &str) -> Option<(Timestamp, String)> {
        let mut it = s.splitn(3, '-');
        let millis = it.next()?.parse().ok()?;
        let counter = it.next()?.parse().ok()?;
        let device = it.next().unwrap_or("").to_string();
        Some((Timestamp { millis, counter }, device))
    }
}

pub fn device_of(hlc: &str) -> String {
    Timestamp::parse(hlc).map(|(_, d)| d).unwrap_or_default()
}

fn wall_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub struct Clock {
    device: String,
    last: Mutex<Timestamp>,
    now_fn: fn() -> u64,
}

impl Clock {
    pub fn new(device: impl Into<String>) -> Self {
        Self::with_time_source(device, wall_millis)
    }

    pub fn with_time_source(device: impl Into<String>, now_fn: fn() -> u64) -> Self {
        Clock {
            device: device.into(),
            last: Mutex::new(Timestamp::default()),
            now_fn,
        }
    }

    pub fn device(&self) -> &str {
        &self.device
    }

    /// A new timestamp for a local event; always greater than anything seen before.
    pub fn now(&self) -> String {
        let wall = (self.now_fn)();
        let mut last = self.last.lock().unwrap();
        if wall > last.millis {
            *last = Timestamp { millis: wall, counter: 0 };
        } else {
            last.counter += 1;
        }
        last.encode(&self.device)
    }

    /// Fold a remote timestamp in, so later local events sort after it.
    pub fn observe(&self, remote: &str) {
        let Some((r, _)) = Timestamp::parse(remote) else { return };
        let wall = (self.now_fn)();
        let mut l = self.last.lock().unwrap();
        let max_m = wall.max(l.millis).max(r.millis);
        let counter = if max_m == l.millis && max_m == r.millis {
            l.counter.max(r.counter) + 1
        } else if max_m == l.millis {
            l.counter + 1
        } else if max_m == r.millis {
            r.counter + 1
        } else {
            0
        };
        *l = Timestamp { millis: max_m, counter };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frozen() -> u64 {
        1_000
    }

    #[test]
    fn monotonic_when_wall_clock_is_frozen() {
        let c = Clock::with_time_source("a", frozen);
        let t1 = c.now();
        let t2 = c.now();
        assert!(t2 > t1);
    }

    #[test]
    fn observe_moves_past_remote() {
        let c = Clock::with_time_source("a", frozen);
        let remote = Timestamp { millis: 5_000, counter: 7 }.encode("b");
        c.observe(&remote);
        assert!(c.now() > remote);
    }

    #[test]
    fn parse_roundtrip_with_uuid_device() {
        let dev = "6f1c-uuid-with-dashes";
        let s = Timestamp { millis: 42, counter: 3 }.encode(dev);
        let (t, d) = Timestamp::parse(&s).unwrap();
        assert_eq!(t, Timestamp { millis: 42, counter: 3 });
        assert_eq!(d, dev);
    }
}
