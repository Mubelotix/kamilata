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

/// Strategies for inserting new routing peeers.
/// 
/// *Inspired by [libp2p::kad::KademliaBucketInserts].*
#[derive(Debug, Clone)]
pub enum RoutingInserts {
    /// Inserts all peers upon connection, as long as the limit is not reached.
    OnConnected,
    /// New peers and addresses are only added to the routing table via explicit calls to [Kamilata::add_address].
    Manual
}

#[derive(Debug, Clone)]
pub struct KamilataConfig {
    /// Min, target and max values in milliseconds
    pub filter_update_delay: MinTargetMax,
    /// Maximum number of filters to manage per peer (default: 8)
    pub max_filter_levels: usize,
    /// Maximum number of peers to manage filters for (default: 20)
    pub max_routing_peers: usize,
    /// Strategy for inserting new routing peers (default: [OnConnected](RoutingInserts::OnConnected)))
    pub routing_inserts: RoutingInserts
}

impl Default for KamilataConfig {
    fn default() -> Self {
        Self {
            filter_update_delay: MinTargetMax { min: 15_000, target: 20_000, max: 60_000*3 },
            max_filter_levels: 8,
            max_routing_peers: 20,
            routing_inserts: RoutingInserts::OnConnected
        }
    }
}
