use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MinTargetMax {
    min: usize,
    target: usize,
    max: usize,
}

impl MinTargetMax {
    pub fn set_min(&mut self, min: usize) {
        self.min = min;
        if self.target < min {
            self.target = min;
        }
        if self.max < min {
            self.max = min;
        }
    }

    pub fn min(&self) -> usize {
        self.min
    }

    pub fn set_max(&mut self, max: usize) {
        self.max = max;
        if self.target > max {
            self.target = max;
        }
        if self.min > max {
            self.min = max;
        }
    }

    pub fn max(&self) -> usize {
        self.max
    }

    pub fn set_target(&mut self, target: usize) {
        self.target = target;
        if self.min > target {
            self.min = target;
        }
        if self.max < target {
            self.max = target;
        }
    }

    pub fn target(&self) -> usize {
        self.target
    }
}

#[derive(Debug, Clone)]
pub struct KamilataConfig {
    /// Min, preferred and max values in milliseconds
    pub filter_update_delay: MinTargetMax,
    /// Min, target and max number of connected peers to maintain
    pub peer_count: MinTargetMax,
}

impl Default for KamilataConfig {
    fn default() -> Self {
        Self {
            filter_update_delay: MinTargetMax { min: 15_000, target: 20_000, max: 60_000*3 },
            peer_count: MinTargetMax { min: 5, target: 6, max: 12 }
        }
    }
}
