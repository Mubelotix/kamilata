use crate::prelude::*;

pub struct FilterDb {
    filters: BTreeMap<PeerId, Vec<Filter<125000>>>,
    our_filters: Vec<Filter<125000>>,
}

impl FilterDb {
    pub fn new() -> FilterDb {
        FilterDb {
            filters: BTreeMap::new(),
            our_filters: Vec::new(),
        }
    }

    pub fn set(&mut self, peer_id: PeerId, filters: Vec<Filter<125000>>) {
        self.filters.insert(peer_id, filters);
    }
}
