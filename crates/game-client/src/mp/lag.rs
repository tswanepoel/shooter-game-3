//! Client RTT from Input seq / Snapshot ack_seq → adaptive remote present delay (029).

use std::collections::VecDeque;

use game_net::Seq;

/// Default remote present delay before any RTT sample (030).
pub const REMOTE_INTERP_DELAY_DEFAULT_SECS: f32 = 0.048;
/// Floor so a lucky ping does not underrun the pose buffer (~4 ticks @ 128 Hz).
pub const REMOTE_INTERP_DELAY_MIN_SECS: f32 = 0.032;
/// Ceiling so spikes do not bury peers too far in the past.
pub const REMOTE_INTERP_DELAY_MAX_SECS: f32 = 0.200;
/// Scale smoothed RTT → view delay (one-way-ish).
const RTT_TO_DELAY_K: f32 = 0.5;
/// Extra cushion on top of scaled RTT (v1: none).
const RTT_JITTER_PAD_SECS: f32 = 0.0;
/// EMA blend toward each new sample: `ema = mix(ema, sample, α)`.
const RTT_EMA_ALPHA: f32 = 0.15;
/// Cap send-time map (same order as predict history).
const INPUT_SEND_TIME_CAP: usize = 256;

/// Tracks Input send times, smoothed RTT, and the live remote interp delay.
#[derive(Debug)]
pub struct LagEstimator {
    /// `(seq, send_time_secs)` for Inputs not yet covered by `ack_seq`.
    send_times: VecDeque<(Seq, f64)>,
    rtt_ema: Option<f32>,
    delay_secs: f32,
}

impl LagEstimator {
    pub fn new() -> Self {
        Self {
            send_times: VecDeque::new(),
            rtt_ema: None,
            delay_secs: REMOTE_INTERP_DELAY_DEFAULT_SECS,
        }
    }

    pub fn clear(&mut self) {
        self.send_times.clear();
        self.rtt_ema = None;
        self.delay_secs = REMOTE_INTERP_DELAY_DEFAULT_SECS;
    }

    pub fn delay_secs(&self) -> f32 {
        self.delay_secs
    }

    pub fn rtt_ema(&self) -> Option<f32> {
        self.rtt_ema
    }

    /// Record wall time when an Input with `seq` is queued for send.
    pub fn note_input_sent(&mut self, seq: Seq, now_secs: f64) {
        self.send_times.push_back((seq, now_secs));
        while self.send_times.len() > INPUT_SEND_TIME_CAP {
            self.send_times.pop_front();
        }
    }

    /// On Snapshot: if `ack_seq` was sent, sample RTT, drop acked stamps, refresh delay.
    pub fn on_ack(&mut self, ack_seq: Seq, now_secs: f64) {
        if let Some((_, send_t)) = self.send_times.iter().find(|(s, _)| *s == ack_seq) {
            let sample = (now_secs - send_t) as f32;
            if sample.is_finite() && sample >= 0.0 {
                self.rtt_ema = Some(match self.rtt_ema {
                    Some(ema) => ema + RTT_EMA_ALPHA * (sample - ema),
                    None => sample,
                });
                self.recompute_delay();
            }
        }
        self.send_times.retain(|(s, _)| *s > ack_seq);
    }

    fn recompute_delay(&mut self) {
        let Some(rtt) = self.rtt_ema else {
            self.delay_secs = REMOTE_INTERP_DELAY_DEFAULT_SECS;
            return;
        };
        let raw = RTT_TO_DELAY_K * rtt + RTT_JITTER_PAD_SECS;
        self.delay_secs = raw.clamp(REMOTE_INTERP_DELAY_MIN_SECS, REMOTE_INTERP_DELAY_MAX_SECS);
    }
}

impl Default for LagEstimator {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance clock in seconds (browser). Falls back to 0 if unavailable.
pub fn client_now_secs() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now() / 1000.0)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_delay_before_samples() {
        let lag = LagEstimator::new();
        assert!((lag.delay_secs() - REMOTE_INTERP_DELAY_DEFAULT_SECS).abs() < 1e-6);
        assert!(lag.rtt_ema().is_none());
    }

    #[test]
    fn ack_samples_rtt_and_clamps_delay() {
        let mut lag = LagEstimator::new();
        lag.note_input_sent(1, 1.0);
        // 40 ms RTT → 0.5 * 0.04 = 0.02 → clamp to MIN 32 ms
        lag.on_ack(1, 1.040);
        let rtt = lag.rtt_ema().unwrap();
        assert!((rtt - 0.040).abs() < 1e-5, "rtt={rtt}");
        assert!(
            (lag.delay_secs() - REMOTE_INTERP_DELAY_MIN_SECS).abs() < 1e-5,
            "delay={}",
            lag.delay_secs()
        );
    }

    #[test]
    fn high_rtt_hits_max_delay() {
        let mut lag = LagEstimator::new();
        // Force EMA high with alpha=1 path: first sample only.
        lag.note_input_sent(1, 0.0);
        lag.on_ack(1, 0.500); // 500 ms RTT → 0.5 * 0.5 = 0.25 → clamp MAX 200 ms
        assert!(
            (lag.delay_secs() - REMOTE_INTERP_DELAY_MAX_SECS).abs() < 1e-5,
            "delay={}",
            lag.delay_secs()
        );
    }

    #[test]
    fn mid_rtt_lands_between_clamps() {
        let mut lag = LagEstimator::new();
        lag.note_input_sent(1, 0.0);
        lag.on_ack(1, 0.240); // 240 ms → 0.5 * 0.24 = 0.12
        assert!(
            (lag.delay_secs() - 0.120).abs() < 1e-4,
            "delay={}",
            lag.delay_secs()
        );
    }

    #[test]
    fn drops_send_times_through_ack() {
        let mut lag = LagEstimator::new();
        lag.note_input_sent(1, 0.0);
        lag.note_input_sent(2, 0.01);
        lag.note_input_sent(3, 0.02);
        lag.on_ack(2, 0.05);
        // seq 1 and 2 dropped; 3 remains
        assert_eq!(lag.send_times.len(), 1);
        assert_eq!(lag.send_times[0].0, 3);
    }

    #[test]
    fn clear_resets() {
        let mut lag = LagEstimator::new();
        lag.note_input_sent(1, 0.0);
        lag.on_ack(1, 0.1);
        lag.clear();
        assert!(lag.rtt_ema().is_none());
        assert!(lag.send_times.is_empty());
        assert!((lag.delay_secs() - REMOTE_INTERP_DELAY_DEFAULT_SECS).abs() < 1e-6);
    }

    #[test]
    fn unknown_ack_does_not_change_delay() {
        let mut lag = LagEstimator::new();
        lag.on_ack(99, 1.0);
        assert!(lag.rtt_ema().is_none());
        assert!((lag.delay_secs() - REMOTE_INTERP_DELAY_DEFAULT_SECS).abs() < 1e-6);
    }
}
