use std::sync::atomic::AtomicU32;

#[derive(Default)]
pub struct Counter {
    value: AtomicU32,
}

impl Counter {
    pub fn new(start: u32) -> Self {
        Counter {
            value: AtomicU32::new(start),
        }
    }

    pub fn next(&self) -> u32 {
        self.value.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}
