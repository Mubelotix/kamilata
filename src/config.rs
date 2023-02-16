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
    /// Min, target and max values in milliseconds
    pub filter_update_delay: MinTargetMax,
    /// Maximum number of filters to manage per peer (default: 8)
    pub max_filter_levels: usize,
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
            filter_update_delay: MinTargetMax { min: 15_000, target: 20_000, max: 60_000*3 },
            max_filter_levels: 8,
            in_routing_peers: MinTargetMax { min: 3, target: 6, max: 20 },
            out_routing_peers: MinTargetMax { min: 3, target: 12, max: 50 },
        }
    }
}
