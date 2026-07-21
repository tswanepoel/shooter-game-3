use game_net::estimated_tick;

#[derive(Debug, Default)]
pub struct ClockSync {
    /// `server_time ≈ local_time + offset`
    offset_secs: Option<f64>,
    min_delay_secs: f64,
    last_delay_secs: Option<f64>,
    last_reply_tick: Option<u64>,
    sample_count: u32,
}

impl ClockSync {
    pub fn new() -> Self {
        Self {
            offset_secs: None,
            min_delay_secs: f64::INFINITY,
            last_delay_secs: None,
            last_reply_tick: None,
            sample_count: 0,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }

    pub fn seed_from_welcome(&mut self, local_secs: f64, server_time_secs: f64, tick: u64) {
        let theta = server_time_secs - local_secs;
        if !theta.is_finite() {
            return;
        }
        self.offset_secs = Some(theta);
        self.last_reply_tick = Some(tick);
    }

    pub fn offset_secs(&self) -> Option<f64> {
        self.offset_secs
    }

    pub fn last_delay_secs(&self) -> Option<f64> {
        self.last_delay_secs
    }

    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    pub fn on_sample(&mut self, t1: f64, t2: f64, t3: f64, t4: f64, reply_tick: u64) {
        let delay = (t4 - t1) - (t3 - t2);
        if !delay.is_finite() || !(0.0..=2.0).contains(&delay) {
            return;
        }
        let theta = ((t2 - t1) + (t3 - t4)) / 2.0;
        if !theta.is_finite() {
            return;
        }

        self.last_delay_secs = Some(delay);
        self.last_reply_tick = Some(reply_tick);
        self.sample_count = self.sample_count.saturating_add(1);

        let is_best = delay <= self.min_delay_secs;
        if is_best {
            self.min_delay_secs = delay;
        }

        match self.offset_secs {
            None => {
                self.offset_secs = Some(theta);
            }
            Some(prev) => {
                let alpha = if is_best {
                    0.45
                } else if delay < self.min_delay_secs * 2.0 {
                    0.12
                } else {
                    0.04
                };
                self.offset_secs = Some(prev + alpha * (theta - prev));
            }
        }
    }

    pub fn estimated_tick(&self, local_secs: f64) -> Option<u64> {
        self.offset_secs.map(|off| estimated_tick(local_secs, off))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_rtt_offset() {
        let mut c = ClockSync::new();
        // Perfect zero delay, server ahead by 5s.
        // t1=1, t2=6, t3=6, t4=1 → delay=0, theta=5
        c.on_sample(1.0, 6.0, 6.0, 1.0, 0);
        let off = c.offset_secs().unwrap();
        assert!((off - 5.0).abs() < 1e-9);
        assert_eq!(c.estimated_tick(1.0), Some(estimated_tick(1.0, 5.0)));
    }

    #[test]
    fn welcome_seed() {
        let mut c = ClockSync::new();
        c.seed_from_welcome(10.0, 15.0, 100);
        assert!((c.offset_secs().unwrap() - 5.0).abs() < 1e-9);
        assert_eq!(c.estimated_tick(10.0), Some(estimated_tick(10.0, 5.0)));
    }
}
