/// Implements timing for the application. Will compute delta times and time steps
/// between frames

use std::time::{Duration, Instant};
pub struct Time {
    last_time : Instant,
    delta_time : Duration
}

impl Time {
    pub fn new(current_instant : Instant) -> Self {
        return Time {
            last_time: current_instant,
            delta_time: Duration::new(0, 0)
        }
    }

    #[inline(always)]
    pub fn delta_seconds(&self) -> f32 {
        return self.delta_time.as_secs_f32();
    }

    #[inline(always)]
    pub fn delta_milliseconds(&self) -> f32 {
        return self.delta_seconds() * 1000.0;
    }

    pub fn step(&mut self, instant : Instant) {
        self.delta_time = instant - self.last_time;
        self.last_time = instant;
    }
}
