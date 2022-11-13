use crate::{prelude::*, Movie};

pub struct Db<Document: self::Document> {
    documents: RwLock<BTreeMap<<Document::SearchResult as SearchResult>::Cid, Document>>,
    filters: RwLock<BTreeMap<PeerId, Vec<Filter<125000>>>>,
    our_root_filter: RwLock<Filter<125000>>,
}

impl<Document: self::Document> Db<Document> {
    pub fn new() -> Db<Document> {
        Db {
            documents: RwLock::new(BTreeMap::new()),
            filters: RwLock::new(BTreeMap::new()),
            our_root_filter: RwLock::new(Filter::new()),
        }
    }

    pub async fn set(&self, peer_id: PeerId, filters: Vec<Filter<125000>>) {
        // TODO size checks
        self.filters.write().await.insert(peer_id, filters);
    }

    pub async fn gen_local_filters(&self, ignore_peers: &[PeerId]) -> Vec<Filter<125000>> {
        let mut local_filters = Vec::new();
        local_filters.push(self.our_root_filter.read().await.clone());

        let mut level = 0;
        let filters = self.filters.read().await;
        loop {
            let mut filter = Filter::new();
            let mut is_null = true;
            for (peer_id, filters) in filters.iter() {
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
