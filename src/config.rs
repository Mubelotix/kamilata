pub enum MinTargetMaxState {
    UnderMin,
    Min,
    UnderTarget,
    Target,
    OverTarget,
    Max,
    OverMax,
}

#[derive(Debug, Clone, protocol::Protocol)]
pub struct MinTargetMax {
    pub(crate) min: u64,
    pub(crate) target: u64,
    pub(crate) max: u64,
}

impl MinTargetMax {
    pub fn set_min(&mut self, min: usize) {
        self.min = min as u64;
        if self.target < self.min {
            self.target = self.min;
        }
        if self.max < self.min {
            self.max = self.min;
        }
    }

    pub fn min(&self) -> usize {
        self.min as usize
    }

    pub fn set_max(&mut self, max: usize) {
        self.max = max as u64;
        if self.target > self.max {
            self.target = self.max;
        }
        if self.min > self.max {
            self.min = self.max;
        }
    }

    pub fn max(&self) -> usize {
        self.max as usize
    }

    pub fn set_target(&mut self, target: usize) {
        self.target = target as u64;
        if self.min > self.target {
            self.min = self.target;
        }
        if self.max < self.target {
            self.max = self.target;
        }
    }

    pub fn target(&self) -> usize {
        self.target as usize
    }

    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        if max < min {
            return None;
        }
        let target = ((self.target + other.target) / 2).clamp(min, max);
        Some(Self { min, target, max })
    }

    pub fn state(&self, value: usize) -> MinTargetMaxState {
        let value = value as u64;
        if value < self.min {
            MinTargetMaxState::UnderMin
        } else if value == self.min {
            MinTargetMaxState::Min
        } else if value < self.target {
            MinTargetMaxState::UnderTarget
        } else if value == self.target {
            MinTargetMaxState::Target
        } else if value < self.max {
            MinTargetMaxState::OverTarget
        } else if value == self.max {
            MinTargetMaxState::Max
        } else {
            MinTargetMaxState::OverMax
        }
    }
}

#[derive(Debug, Clone)]
pub struct KamilataConfig {
    /// Min, target and max values in milliseconds
    pub get_filters_interval: MinTargetMax,
    /// Maximum number of filters to manage per peer (default: 8)
    pub filter_count: usize,
    /// Number of peers we receive filters from (default: 3,8,20)
    /// 
    /// New peers will automatically be dialed until the `min` value is reached.
    /// Other peers will be requested to send us their filters until the `target` value is reached.
    /// Peers requesting us to receive their filters will be accepted until the `max` value is reached.
    /// We will stop receiving filters from peers if the `max` value is overreached.
    pub in_routing_peers: MinTargetMax,
    /// Number of peers we send filters to (default: 4,16,50)
    /// 
    /// New peers will automatically be dialed until the `min` value is reached.
    /// Other peers will be requested to receive our filters until the `target` value is reached.
    /// Peers requesting us our filters will be accepted until the `max` value is reached.
    /// We will stop sending filters to peers if the `max` value is overreached.
    pub out_routing_peers: MinTargetMax,
}

impl Default for KamilataConfig {
    fn default() -> Self {
        Self {
            get_filters_interval: MinTargetMax { min: 15_000, target: 20_000, max: 60_000*3 },
            filter_count: 8,
            in_routing_peers: MinTargetMax { min: 3, target: 6, max: 20 },
            out_routing_peers: MinTargetMax { min: 3, target: 12, max: 50 },
        }
    }
}
