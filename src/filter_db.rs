use crate::prelude::*;

pub struct FilterDb {
    filters: BTreeMap<PeerId, Vec<Filter<125000>>>,
    our_root_filter: Filter<125000>,
}

impl FilterDb {
    pub fn new() -> FilterDb {
        FilterDb {
            filters: BTreeMap::new(),
            our_root_filter: Filter::new(),
        }
    }

    pub fn set(&mut self, peer_id: PeerId, filters: Vec<Filter<125000>>) {
        // TODO size checks
        self.filters.insert(peer_id, filters);
    }

    pub fn gen_local_filters(&self, ignore_peers: &[PeerId]) -> Vec<Filter<125000>> {
        let mut local_filters = Vec::new();
        local_filters.push(self.our_root_filter.clone());

        let mut level = 0;
        loop {
            let mut filter = Filter::new();
            let mut is_null = true;
            for (peer_id, filters) in self.filters.iter() {
                if ignore_peers.contains(peer_id) {
                    continue;
                }
                if let Some(f) = filters.get(level) {
                    filter.bitor_assign_ref(f);
                    is_null = false;
                }
            }
            match is_null {
                true => break,
                false => {
                    local_filters.push(filter);
                    level += 1;
                }
            }
        }

        local_filters
    }
}
