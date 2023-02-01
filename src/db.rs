use std::ops::DerefMut;
use crate::prelude::*;

pub struct Db<const N: usize, D: Document<N>> {
    // In order to prevent deadlocks, please lock the different fields in the same order as they are declared in the struct.

    documents: RwLock<BTreeMap<<D::SearchResult as SearchResult>::Cid, D>>,
    filters: RwLock<BTreeMap<PeerId, Vec<Filter<N>>>>,
    our_root_filter: RwLock<Filter<N>>,
}

impl<const N: usize, D: Document<N>> Db<N, D> {
    pub fn new() -> Db<N, D> {
        Db {
            documents: RwLock::new(BTreeMap::new()),
            filters: RwLock::new(BTreeMap::new()),
            our_root_filter: RwLock::new(Filter::new()),
        }
    }

    /// Inserts a single document to the database.
    /// This will update the root filter without fully recompting it.
    pub async fn insert_document(&self, document: D) {
        document.apply_to_filter(self.our_root_filter.write().await.deref_mut());
        self.documents.write().await.insert(document.cid().clone(), document);
    }

    /// Inserts multiple documents to the database.
    /// The database while be locked for the duration of the insertion.
    /// Use [`insert_document`] if you want to allow access to other threads while inserting.
    pub async fn insert_documents(&self, documents: Vec<D>) {
        let new_documents = documents;
        let mut documents = self.documents.write().await;
        let mut our_root_filter = self.our_root_filter.write().await;
        for document in new_documents {
            document.apply_to_filter(our_root_filter.deref_mut());
            documents.insert(document.cid().clone(), document);
        }
    }

    /// Removes all documents from the database.
    pub async fn clear_documents(&self) {
        *self.our_root_filter.write().await = Filter::new();
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
            *self.our_root_filter.write().await = new_root_filter;
        }
    }

    pub async fn set_remote_filter(&self, peer_id: PeerId, filters: Vec<Filter<N>>) {
        // TODO size checks
        self.filters.write().await.insert(peer_id, filters);
    }

    pub async fn gen_local_filters(&self, ignore_peers: &[PeerId]) -> Vec<Filter<N>> {
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
