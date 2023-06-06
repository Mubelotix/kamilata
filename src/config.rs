#[derive(Debug, Clone, protocol::Protocol)]
pub struct MinTargetMax {
    pub(crate) min: u64,
    pub(crate) target: u64,
    pub(crate) max: u64,
}

impl MinTargetMax {
    pub const fn new(min: usize, target: usize, max: usize) -> Self {
        // TODO checks
        Self {
            min: min as u64,
            target: target as u64,
            max: max as u64,
        }
    }

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

    pub fn is_under_target(&self, value: usize) -> bool {
        let value = value as u64;
        value < self.target
    }

    pub fn is_max_or_over(&self, value: usize) -> bool {
        let value = value as u64;
        value >= self.max
    }
}

#[derive(Debug, Clone)]
pub struct KamilataConfig {
    /// Min, target and max values in milliseconds
    pub get_filters_interval: MinTargetMax,
    /// Maximum number of filters to manage per peer (default: 8)
    pub filter_count: usize,
    /// Maximum number of peers we receive filters from (default: 20)
    pub max_seeders: usize,
    /// Maximum number of peers we send filters to (default: 50)
    pub max_leechers: usize,
    /// Automatically leech peers (default: true)
    pub auto_leech: bool,
}

impl Default for KamilataConfig {
    fn default() -> Self {
        Self {
            get_filters_interval: MinTargetMax { min: 15_000, target: 20_000, max: 60_000*3 },
            filter_count: 8,
            max_seeders: 20,
            max_leechers: 50,
            auto_leech: true,
        }
    }
}
