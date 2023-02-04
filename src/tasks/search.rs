//! This module contains the algorithm that is used for discovering results on the network.

use super::*;
use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq)]
struct QueryList {
    queries: Vec<Option<usize>>,
}

impl QueryList {
    fn min_dist(&self) -> Option<(usize, usize)> {
        let mut result = None;
        for (query, dist) in self.queries.iter().enumerate() {
            if let Some(dist) = dist {
                match result {
                    None => result = Some((query, *dist)),
                    Some((_, min_dist)) => {
                        if *dist < min_dist {
                            result = Some((query, *dist));
                        }
                    }
                }
            }
        }
        result
    }
}

impl std::cmp::Ord for QueryList {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.min_dist(), other.min_dist()) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some((min_dist, min_dist_query)), Some((other_min_dist, other_min_dist_query))) => match min_dist.cmp(&other_min_dist) {
                Ordering::Equal => min_dist_query.cmp(&other_min_dist_query).reverse(),
                Ordering::Less => Ordering::Greater,
                Ordering::Greater => Ordering::Less,
            },
        }
    }
}

impl std::cmp::PartialOrd for QueryList {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

async fn search_one<const N: usize, D: Document<N>>(peer_id: PeerId) -> (PeerId, Vec<(D::SearchResult, usize)>,  Vec<(PeerId, Vec<Option<usize>>)>) {
    todo!()
}
 
pub async fn search<const N: usize, D: Document<N>>(
    our_peer_id: PeerId,
    search_follower: OngoingSearchFollower<D::SearchResult>,
    handler_messager: HandlerMessager,
    db: Arc<Db<N, D>>,
) -> TaskOutput {
    // Extract settings
    let req_limit = search_follower.req_limit().await;

    // Query ourselves
    let queries = search_follower.queries().await;
    let local_results = db.search_local(&queries).await;
    for (result, query) in local_results {
        search_follower.send((result, query, our_peer_id)).await.unwrap();
    }
    let queries_hashed = queries
        .into_iter()
        .map(|(words, n)| (
            words.into_iter().map(|w| D::WordHasher::hash_word(w.as_str())).collect::<Vec<_>>(), n
        ))
        .collect::<Vec<_>>();
    let remote_results = db.search_remote(&queries_hashed).await;
    let mut providers = BinaryHeap::new();
    let mut already_queried = HashSet::new();
    for (peer_id, queries) in remote_results {
        providers.push((peer_id, QueryList { queries }));
    }

    // Keep querying new peers for new results
    let mut ongoing_requests = Vec::new();
    'search: loop {
        // Start new requests until limit is reached
        while ongoing_requests.len() < req_limit {
            let Some((peer_id, queries)) = providers.pop() else {break 'search};
            already_queried.insert(peer_id);
            ongoing_requests.push(Box::pin(search_one::<N,D>(peer_id)));
        }

        // Wait for one of the ongoing requests to finish
        let ((peer_id, local_results, remote_results), _, remaining_requests) = futures::future::select_all(ongoing_requests).await;
        ongoing_requests = remaining_requests;
        for (result, query) in local_results {
            search_follower.send((result, query, peer_id)).await.unwrap();
        }
        for (peer_id, queries) in remote_results {
            if !already_queried.contains(&peer_id) {
                providers.push((peer_id, QueryList { queries }));
            }
        }
    }

    TaskOutput::None
}
