//! This module contains the algorithm that is used for discovering results on the network.

use futures::future::join_all;

use super::*;
use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;

// Constants for each [FixedSearchPriority]. They are used as const generics.
const ANY: usize = 0;
const SPEED: usize = 1;
const RELEVANCE: usize = 2;

/// Information about a provider, generic over the priority of the search.
/// The priority will determine the implementation of [Ord] on this struct.
#[derive(Debug, PartialEq, Eq)]
struct ProviderInfo<const PRIORITY: usize> {
    peer_id: PeerId,
    queries: Vec<Option<usize>>,
    addresses: Vec<Multiaddr>,
}

/// A trait allowing APIs over any [ProviderInfo], regardless of the priority.
trait AnyProviderInfo: Sized {
    fn into_parts(self) -> (PeerId, Vec<Option<usize>>, Vec<Multiaddr>);
    fn into_whatever(self) -> ProviderInfo<ANY> {
        let (peer_id, queries, addresses) = self.into_parts();
        ProviderInfo { peer_id, queries, addresses }
    }
    fn into_speed(self) -> ProviderInfo<SPEED> {
        let (peer_id, queries, addresses) = self.into_parts();
        ProviderInfo { peer_id, queries, addresses }
    }
    fn into_relevance(self) -> ProviderInfo<RELEVANCE> {
        let (peer_id, queries, addresses) = self.into_parts();
        ProviderInfo { peer_id, queries, addresses }
    }
}

impl AnyProviderInfo for ProviderInfo<ANY> {
    fn into_parts(self) -> (PeerId, Vec<Option<usize>>, Vec<Multiaddr>) {
        (self.peer_id, self.queries, self.addresses)
    }
}
impl AnyProviderInfo for ProviderInfo<SPEED> {
    fn into_parts(self) -> (PeerId, Vec<Option<usize>>, Vec<Multiaddr>) {
        (self.peer_id, self.queries, self.addresses)
    }
}
impl AnyProviderInfo for ProviderInfo<RELEVANCE> {
    fn into_parts(self) -> (PeerId, Vec<Option<usize>>, Vec<Multiaddr>) {
        (self.peer_id, self.queries, self.addresses)
    }
}
impl AnyProviderInfo for (PeerId, Vec<Option<usize>>, Vec<Multiaddr>) {
    fn into_parts(self) -> (PeerId, Vec<Option<usize>>, Vec<Multiaddr>) {
        self
    }
}

/// A [BinaryHeap] that can change its way of ordering its elements.
enum ProviderBinaryHeap {
    Speed(BinaryHeap<ProviderInfo<SPEED>>),
    Relevance(BinaryHeap<ProviderInfo<RELEVANCE>>),
}

impl ProviderBinaryHeap {
    fn push(&mut self, provider: impl AnyProviderInfo) {
        match self {
            ProviderBinaryHeap::Speed(heap) => heap.push(provider.into_speed()),
            ProviderBinaryHeap::Relevance(heap) => heap.push(provider.into_relevance()),
        }
    }

    fn pop(&mut self) -> Option<ProviderInfo<ANY>> {
        match self {
            ProviderBinaryHeap::Speed(heap) => heap.pop().map(|provider| provider.into_whatever()),
            ProviderBinaryHeap::Relevance(heap) => heap.pop().map(|provider| provider.into_whatever()),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            ProviderBinaryHeap::Speed(heap) => heap.is_empty(),
            ProviderBinaryHeap::Relevance(heap) => heap.is_empty(),
        }
    }

    fn update_priority(&mut self, priority: SearchPriority, documents_found: usize) {
        match priority.get_priority(documents_found) {
            FixedSearchPriority::Speed => match self {
                ProviderBinaryHeap::Speed(_) => (),
                ProviderBinaryHeap::Relevance(heap) => {
                    let mut new_heap = BinaryHeap::new();
                    for provider in heap.drain() {
                        new_heap.push(provider.into_speed());
                    }
                    *self = ProviderBinaryHeap::Speed(new_heap);
                }
            },
            FixedSearchPriority::Relevance => match self {
                ProviderBinaryHeap::Speed(heap) => {
                    let mut new_heap = BinaryHeap::new();
                    for provider in heap.drain() {
                        new_heap.push(provider.into_relevance());
                    }
                    *self = ProviderBinaryHeap::Relevance(new_heap);
                }
                ProviderBinaryHeap::Relevance(_) => (),
            },
        }
    }
}

impl<const PRIORITY: usize> ProviderInfo<PRIORITY> {
    fn nearest(&self) -> Option<(usize, usize)> {
        let mut nearest = None;
        for (query, dist) in self.queries.iter().enumerate() {
            if let Some(dist) = dist {
                match nearest {
                    None => nearest = Some((query, *dist)),
                    Some((_, min_dist)) => {
                        if *dist < min_dist {
                            nearest = Some((query, *dist));
                        }
                    }
                }
            }
        }
        nearest
    }

    fn best(&self) -> Option<(usize, usize)> {
        for (query, dist) in self.queries.iter().enumerate() {
            if let Some(dist) = dist {
                return Some((query, *dist))
            }
        }
        None
    }
}

impl std::cmp::Ord for ProviderInfo<SPEED> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.nearest(), other.nearest()) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some((query1, dist1)), Some((query2, dist2))) => match dist1.cmp(&dist2) {
                Ordering::Equal => query1.cmp(&query2).reverse(),
                Ordering::Less => Ordering::Greater,
                Ordering::Greater => Ordering::Less,
            },
        }
    }
}

impl std::cmp::PartialOrd for ProviderInfo<SPEED> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl std::cmp::Ord for ProviderInfo<RELEVANCE> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.best(), other.best()) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some((query1, dist1)), Some((query2, dist2))) => match query1.cmp(&query2) {
                Ordering::Equal => dist1.cmp(&dist2).reverse(),
                Ordering::Less => Ordering::Greater,
                Ordering::Greater => Ordering::Less,
            },
        }
    }
}

impl std::cmp::PartialOrd for ProviderInfo<RELEVANCE> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

struct QueryResult<S: SearchResult> {
    result: S,
    query: usize,
}

async fn search_one<const N: usize, S: Store<N>>(
    queries: SearchQueries,
    behavior_controller: BehaviourController,
    addresses: Vec<Multiaddr>,
    our_peer_id: PeerId,
    remote_peer_id: PeerId,
) -> Option<(PeerId, Vec<QueryResult<S::SearchResult>>,  Vec<ProviderInfo<ANY>>)> {
    debug!("{our_peer_id} Querying {remote_peer_id} for results");

    // Dial the peer, orders the handle to request it, and wait for the response
    let request = RequestPacket::Search(SearchPacket {
        queries: queries.inner.into_iter().map(|(words, min_matching)| Query {
            words,
            min_matching: min_matching as u16
        }).collect()
    });
    let (sender, receiver) = oneshot_channel();
    behavior_controller.dial_peer_and_message(remote_peer_id, addresses, HandlerInEvent::Request {
        request,
        sender,
    }).await;
    let response = match receiver.await {
        Ok(response) => response,
        Err(_) => {
            warn!("{our_peer_id} Failed to receive response from {remote_peer_id}");
            return None;
        },
    };

    // Process the response
    let (matches, distant_matches) = match response {
        Some(ResponsePacket::Results(ResultsPacket { distant_matches, matches })) => (matches, distant_matches),
        Some(_) => panic!("Unexpected response type"),
        None => {
            warn!("{our_peer_id} Received no response from {remote_peer_id}");
            return None;
        }
    };

    let query_results = matches.into_iter().map(|local_match|
        QueryResult {
            result: S::SearchResult::from_bytes(&local_match.result),
            query: local_match.query as usize,
        }
    ).collect::<Vec<_>>();

    let route_to_results = distant_matches.into_iter().map(|distant_match|
        ProviderInfo {
            peer_id: distant_match.peer_id.into(),
            queries: distant_match.queries.into_iter().map(|m| m.map(|m| m as usize)).collect(),
            addresses: distant_match.addresses.into_iter().filter_map(|a| a.parse().ok()).collect(),
        }
    ).collect::<Vec<_>>();

    debug!("{our_peer_id} Received {} results and {} routes from {remote_peer_id}", query_results.len(), route_to_results.len());

    Some((remote_peer_id, query_results, route_to_results))
}
 
pub(crate) async fn search<const N: usize, S: Store<N>>(
    search_follower: OngoingSearchFollower<S::SearchResult>,
    behavior_controller: BehaviourController,
    db: Arc<Db<N, S>>,
    our_peer_id: PeerId,
) -> TaskOutput {
    info!("{our_peer_id} Starting search task");

    // Query ourselves
    let queries = search_follower.queries().await;
    let local_results = join_all(queries.inner.clone().into_iter().map(|(words, min_matching)| db.store().search(words, min_matching))).await;
    let mut cids = HashSet::new();
    for (query, results) in local_results.into_iter().enumerate() {
        for result in results {
            if cids.insert(result.cid()) {
                let _ = search_follower.send((result, query, our_peer_id)).await;
            }
        }
    }
    let queries_hashed = queries
        .inner
        .clone()
        .into_iter()
        .map(|(words, n)| (
            words.into_iter().map(|w| S::hash_word(w.as_str())).collect::<Vec<_>>(), n
        ))
        .collect::<Vec<_>>();
    let remote_results = db.search_routes(&queries_hashed).await;
    let mut config;
    let mut providers = ProviderBinaryHeap::Speed(BinaryHeap::new());
    let mut already_queried = HashSet::new();
    let mut final_peers = 0;
    let mut documents_found = 0;
    for (peer_id, queries) in remote_results {
        providers.push((peer_id, queries, Vec::new()));
    }

    // Keep querying new peers for new results
    let mut ongoing_requests = Vec::new();
    loop {
        search_follower.set_query_counts(already_queried.len(), final_peers, ongoing_requests.len()).await;
        config = search_follower.config().await;
        providers.update_priority(config.priority, documents_found);

        // TODO: update queries

        // Start new requests until limit is reached
        while ongoing_requests.len() < config.req_limit {
            let Some(provider) = providers.pop() else {break};
            already_queried.insert(provider.peer_id);
            let search = search_one::<N,S>(queries.clone(), behavior_controller.clone(), provider.addresses, our_peer_id, provider.peer_id);
            ongoing_requests.push(Box::pin(timeout(Duration::from_millis(config.timeout_ms as u64), search)));
        }

        // Ends the loop when no more requests can be made
        if providers.is_empty() && ongoing_requests.is_empty() {
            break;
        }

        // Wait for one of the ongoing requests to finish
        let (r, _, remaining_requests) = futures::future::select_all(ongoing_requests).await;
        ongoing_requests = remaining_requests;
        let (peer_id, query_results, routes) = match r {
            Ok(Some(r)) => r,
            Ok(None) => continue,
            Err(_) => {
                warn!("{our_peer_id} Search request timed out");
                continue
            },
        };
        if !query_results.is_empty() {
            final_peers += 1;
        }
        documents_found += query_results.len(); // TODO: make sure it's not too easy for a malicious peer to make this increase too much
        for query_result in query_results {
            let r = search_follower.send((query_result.result, query_result.query, peer_id)).await;
            if r.is_err() {
                warn!("{our_peer_id} Search interrupted due to results being dropped");
                return TaskOutput::None;
            }
        }
        for route in routes {
            if !already_queried.contains(&route.peer_id) && !route.addresses.is_empty() {
                providers.push(route);
            }
        }
    }

    info!("{our_peer_id} Search task finished");
    
    TaskOutput::None
}
