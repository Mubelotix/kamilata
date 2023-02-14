use std::ops::DerefMut;
use crate::prelude::*;

pub struct Db<const N: usize, D: Document<N>> {
    // In order to prevent deadlocks, please lock the different fields in the same order as they are declared in the struct.

    documents: RwLock<BTreeMap<<D::SearchResult as SearchResult>::Cid, D>>,
    filters: RwLock<BTreeMap<PeerId, Vec<Filter<N>>>>,
    addresses: RwLock<BTreeMap<PeerId, Vec<Multiaddr>>>,
    root_filter: RwLock<Filter<N>>,
}

impl<const N: usize, D: Document<N>> Default for Db<N, D> {
    fn default() -> Self {
        Db {
            documents: RwLock::new(BTreeMap::new()),
            filters: RwLock::new(BTreeMap::new()),
            addresses: RwLock::new(BTreeMap::new()),
            root_filter: RwLock::new(Filter::new()),
        }
    }
}

impl<const N: usize, D: Document<N>> Db<N, D> {
    /// Inserts a single document to the database.
    /// This will update the root filter without fully recompting it.
    pub async fn insert_document(&self, document: D) {
        document.apply_to_filter(self.root_filter.write().await.deref_mut());
        self.documents.write().await.insert(document.cid().clone(), document);
    }

    /// Inserts multiple documents to the database.
    /// The database while be locked for the duration of the insertion.
    /// Use [`insert_document`] if you want to allow access to other threads while inserting.
    pub async fn insert_documents(&self, documents: Vec<D>) {
        let new_documents = documents;
        let mut documents = self.documents.write().await;
        let mut root_filter = self.root_filter.write().await;
        for document in new_documents {
            document.apply_to_filter(root_filter.deref_mut());
            documents.insert(document.cid().clone(), document);
        }
    }

    /// Removes all documents from the database.
    pub async fn clear_documents(&self) {
        *self.root_filter.write().await = Filter::new();
        self.documents.write().await.clear();
    }

    /// Removes a single document from the index.
    /// This is expensive as it will recompute the root filter.
    /// 
    /// If you want to remove all documents, use `clear_documents` instead.
    /// If you want to remove multiple documents, use `remove_documents` instead.
    pub async fn remove_document(&self, cid: &<D::SearchResult as SearchResult>::Cid) {
        self.remove_documents(&[cid]).await;
    }

    /// Removes multiple documents from the index.
    /// This is expensive as it will recompute the root filter.
    pub async fn remove_documents(&self, cids: &[&<D::SearchResult as SearchResult>::Cid]) {
        let mut recompute_root_filter = false;
        let mut documents = self.documents.write().await;
        for cid in cids {
            if documents.remove(cid).is_some() {
                recompute_root_filter = true;
            }
        }

        let documents = documents.downgrade();
        if recompute_root_filter {
            let mut new_root_filter = Filter::new();
            for document in documents.values() {
                document.apply_to_filter(&mut new_root_filter);
            }
            drop(documents);
            *self.root_filter.write().await = new_root_filter;
        }
    }

    pub async fn set_remote_filter(&self, peer_id: PeerId, filters: Vec<Filter<N>>) {
        // TODO size checks
        self.filters.write().await.insert(peer_id, filters);
    }

    pub(crate) async fn get_filters(&self, ignore_peers: &[PeerId]) -> Vec<Filter<N>> {
        let mut result = Vec::new();
        result.push(self.root_filter.read().await.clone());

        let filters = self.filters.read().await;
        for level in 1..10 {
            let mut filter = Filter::new();
            let mut is_null = true;
            for (peer_id, filters) in filters.iter() {
                if let Some(f) = filters.get(level-1) {
                    filter.bitor_assign_ref(f);
                    is_null = false;
                }
            }
            match is_null {
                true => break,
                false => result.push(filter),
            }
        }

        result
    }

    pub(crate) async fn get_filters_bytes(&self, ignore_peers: &[PeerId]) -> Vec<Vec<u8>> {
        let filters = self.get_filters(ignore_peers).await;
        filters.into_iter().map(|f| <Vec<u8>>::from(&f)).collect()
    }

    /// Adds a new address for a peer.
    pub async fn insert_address(&self, peer_id: PeerId, addr: Multiaddr, front: bool) {
        let mut addresses = self.addresses.write().await;
        let addresses = addresses.entry(peer_id).or_insert_with(Vec::new);
        if !addresses.contains(&addr) {
            match front {
                true => addresses.insert(0, addr),
                false => addresses.push(addr),
            }
        }
    }

    /// Gets the addresses we know for a peer, ordered by how well they are expected to work.
    pub async fn get_addresses(&self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.addresses.read().await.get(peer_id).cloned().unwrap_or_default()
    }

    /// Returns peers and their distance to each query.
    /// The distance from node `n` for query `i` can be found at index `i` in the array associated with `n`.
    pub async fn search_remote(&self, hashed_queries: &Vec<(Vec<usize>, usize)>) -> Vec<(PeerId, Vec<Option<usize>>)> {
        let filters = self.filters.read().await;
        filters
            .iter()
            .map(|(peer_id, filters)| {
                let mut matches = Vec::new();
                for (query_id, (hashed_words, min_matching)) in hashed_queries.iter().enumerate() {
                    let mut best_distance = None;
                    for (distance, filter) in filters.iter().enumerate() {
                        if hashed_words.iter().filter(|w| filter.get_bit(**w)).count() >= *min_matching {
                            best_distance = Some(distance);
                            break;
                        }
                    }
                    matches.push(best_distance)
                }
                (*peer_id, matches)
            })
            .filter(|(_,m)| !m.is_empty())
            .collect()
    }

    pub async fn search_local(&self, queries: &SearchQueries) -> Vec<(D::SearchResult, usize)> {
        let documents = self.documents.read().await;
        documents.values().filter_map(|document| {
            for (query_id, (words, min_matching)) in queries.inner.iter().enumerate() {
                if let Some(search_result) = document.search_result(words, *min_matching) {
                    return Some((search_result, query_id));
                }
            }
            None
        }).collect()
    }
}
