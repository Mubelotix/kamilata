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

struct QueryResult<S: SearchResult> {
    result: S,
    query: usize,
}

struct RouteToResults {
    next: PeerId,
    queries: QueryList,
}

async fn search_one<const N: usize, D: Document<N>>(
    queries: Vec<(Vec<String>, usize)>,
    behavior_controller: BehaviourController,
    our_peer_id: PeerId,
    remote_peer_id: PeerId,
) -> (PeerId, Vec<QueryResult<D::SearchResult>>,  Vec<RouteToResults>) {
    // Dial the peer, orders the handle to request it, and wait for the response
    let request = RequestPacket::Search(SearchPacket {
        queries: queries.into_iter().map(|(words, min_matching)| Query {
            words,
            min_matching: min_matching as u16
        }).collect()
    });
    let (pending_task, receiver) = pending_request::<N, D>(request, our_peer_id, remote_peer_id);
    behavior_controller.dial_peer_and_message(remote_peer_id, HandlerInEvent::AddPendingTask(pending_task)).await;
    let response = receiver.await.unwrap();

    // Process the response
    let (matches, distant_matches) = match response {
        ResponsePacket::Results(ResultsPacket { distant_matches, matches }) => (matches, distant_matches),
        _ => panic!("Unexpected response type"),
    };

    let query_results = matches.into_iter().map(|local_match|
        QueryResult {
            result: D::SearchResult::from_bytes(&local_match.result),
            query: local_match.query as usize,
        }
    ).collect::<Vec<_>>();

    let route_to_results = distant_matches.into_iter().map(|distant_match|
        RouteToResults {
            next: distant_match.peer_id.into(),
            queries: QueryList {
                queries: distant_match.queries.into_iter().map(|m| m.map(|m| m as usize)).collect()
            },
        }
    ).collect::<Vec<_>>();

    (remote_peer_id, query_results, route_to_results)
}
 
pub async fn search<const N: usize, D: Document<N>>(
    search_follower: OngoingSearchFollower<D::SearchResult>,
    behavior_controller: BehaviourController,
    db: Arc<Db<N, D>>,
    our_peer_id: PeerId,
) -> TaskOutput {
    // Extract settings
    let req_limit = search_follower.req_limit().await;
    let timeout_ms = search_follower.timeout_ms().await as u64;
    // TODO: test

    // Query ourselves
    let queries = search_follower.queries().await;
    let local_results = db.search_local(&queries).await;
    for (result, query) in local_results {
        search_follower.send((result, query, our_peer_id)).await.unwrap();
    }
    let queries_hashed = queries
        .clone()
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
        // TODO: update queries

        // Start new requests until limit is reached
        while ongoing_requests.len() < req_limit {
            let Some((remote_peer_id, _queries)) = providers.pop() else {break 'search};
            already_queried.insert(remote_peer_id);
            let search = search_one::<N,D>(queries.clone(), behavior_controller.clone(), our_peer_id, remote_peer_id);
            ongoing_requests.push(Box::pin(timeout(Duration::from_millis(timeout_ms), search)));
        }

        // Wait for one of the ongoing requests to finish
        let (r, _, remaining_requests) = futures::future::select_all(ongoing_requests).await;
        ongoing_requests = remaining_requests;
        let (peer_id, query_results, routes_to_results) = match r {
            Ok(r) => r,
            Err(_) => continue,
        };
        for query_result in query_results {
            search_follower.send((query_result.result, query_result.query, peer_id)).await.unwrap();
        }
        for route_to_results in routes_to_results {
            if !already_queried.contains(&peer_id) {
                providers.push((route_to_results.next, route_to_results.queries));
            }
        }
    }

    TaskOutput::None
}
