use std::collections::BTreeSet;
use crate::prelude::*;

pub(crate) struct Db<const N: usize, S: Store<N>> {
    // In order to prevent deadlocks, please lock the different fields in the same order as they are declared in the struct.

    config: RwLock<KamilataConfig>,
    /// Documents to add in the global network corpus
    store: S,
    /// Filters received from other peers
    filters: RwLock<BTreeMap<PeerId, Vec<Filter<N>>>>,
    /// Peers we send filters to
    leechers: RwLock<BTreeSet<PeerId>>,
    /// Known addresses of peers that are connected to us
    addresses: RwLock<BTreeMap<PeerId, Vec<Multiaddr>>>,
}

impl<const N: usize, S: Store<N>> Db<N, S> {
    pub fn new(config: KamilataConfig, store: S) -> Self {
        Db {
            config: RwLock::new(config),
            store,
            filters: RwLock::new(BTreeMap::new()),
            addresses: RwLock::new(BTreeMap::new()),
            leechers: RwLock::new(BTreeSet::new()),
        }
    }

    pub async fn get_config(&self) -> KamilataConfig {
        self.config.read().await.clone()
    }

    pub async fn set_config(&self, config: KamilataConfig) {
        *self.config.write().await = config;
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    pub async fn seeder_count(&self) -> usize {
        self.filters.read().await.len()
    }

    pub async fn leecher_count(&self) -> usize {
        self.leechers.read().await.len()
    }

    /// Remove data about a peer.
    pub async fn remove_peer(&self, peer_id: &PeerId) {
        self.filters.write().await.remove(peer_id);
        self.addresses.write().await.remove(peer_id);
        self.leechers.write().await.remove(peer_id);
    }

    /// Sets a peer as a out_routing_peer
    pub async fn add_leecher(&self, peer_id: PeerId) -> Result<(), TooManyOutRoutingPeers> {
        let config = self.config.read().await;
        let mut leachers = self.leechers.write().await;
        if leachers.len() < config.max_leechers {
            leachers.insert(peer_id);
            Ok(())
        } else {
            Err(TooManyOutRoutingPeers{})
        }
    }

    pub async fn set_remote_filter(&self, peer_id: PeerId, filters: Vec<Filter<N>>) {
        // TODO size checks
        self.filters.write().await.insert(peer_id, filters);
    }

    pub(crate) async fn get_filters(&self, ignore_peers: &[PeerId]) -> Vec<Filter<N>> {
        let mut result = Vec::new();
        result.push(self.store.get_filter().await); // FIXME: This is slow

        let filters = self.filters.read().await;
        for level in 1..10 {
            let mut filter = Filter::new();
            let mut is_null = true;
            for (peer_id, filters) in filters.iter() {
                if ignore_peers.contains(peer_id) {
                    continue;
                }
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
    pub async fn search_routes(&self, hashed_queries: &[(Vec<usize>, usize)]) -> Vec<(PeerId, Vec<Option<usize>>)> {
        let filters = self.filters.read().await;
        filters
            .iter()
            .map(|(peer_id, filters)| {
                (*peer_id, hashed_queries.iter().map(|(hashed_words, min_matching)| {
                    let mut best_distance = None;
                    for (distance, filter) in filters.iter().enumerate() {
                        if hashed_words.iter().filter(|w| filter.get_bit(**w)).count() >= *min_matching {
                            best_distance = Some(distance);
                            break;
                        }
                    }
                    best_distance
                }).collect::<Vec<_>>())
            })
            .filter(|(_,m)| m.iter().any(|d| d.is_some()))
            .collect()
    }

    /*
    
    /// Returns peers and their distance to the query.
    pub async fn search_routes(&self, hashed_words: Vec<usize>, min_matching: usize) -> Vec<(PeerId, usize)> {
        let filters = self.filters.read().await;
        filters
            .iter()
            .filter_map(|(peer_id, filters)|
                filters.iter().position(|f| {
                    hashed_words.iter().filter(|w| f.get_bit(**w)).count() >= min_matching
                })
                .map(|d| (*peer_id, d))
            )
            .collect()
    }

     */
}

/// Error returned when we try to add a new outgoing routing peer but there are already too many.
#[derive(Debug, Clone)]
pub struct TooManyOutRoutingPeers {}
