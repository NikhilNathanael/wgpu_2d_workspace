use std::time::Instant;

pub struct Timer {
    start_time: Instant,
    last_reset: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            last_reset: Instant::now(),
        }
    }

    pub fn reset(&mut self) {
        self.last_reset = Instant::now();
    }

    pub fn elapsed_start(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }

    pub fn elapsed_reset(&self) -> f32 {
        self.last_reset.elapsed().as_secs_f32()
    }
}
