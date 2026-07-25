//! Shared server clock and session id allocator.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

use game_net::PlayerId;

pub struct SharedClock {
    epoch: Instant,
    tick: AtomicU64,
}

impl SharedClock {
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
            tick: AtomicU64::new(0),
        }
    }

    pub fn server_time_secs(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64()
    }

    pub fn tick(&self) -> u64 {
        self.tick.load(Ordering::Relaxed)
    }

    pub fn advance(&self) {
        self.tick.fetch_add(1, Ordering::Relaxed);
    }
}

pub struct IdAllocator {
    next: AtomicU32,
}

impl IdAllocator {
    pub fn new() -> Self {
        Self {
            next: AtomicU32::new(1),
        }
    }

    pub fn alloc(&self) -> PlayerId {
        self.next.fetch_add(1, Ordering::Relaxed)
    }
}
