//! This module contains the handler of requests from remote peers.

use super::*;

pub(crate) async fn handle_request<const N: usize, D: Document<N>>(mut stream: KamInStreamSink<NegotiatedSubstream>, db: Arc<Db<N, D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> HandlerTaskOutput {
    let request = match stream.next().await {
        Some(Ok(request)) => request,
        Some(Err(e)) => {
            error!("{our_peer_id} Error while receiving request from {remote_peer_id}: {e}");
            return HandlerTaskOutput::None;
        },
        None => return HandlerTaskOutput::None,
    };

    debug!("{our_peer_id} Received a request: {request:?}");
    match request {
        RequestPacket::GetFilters(refresh_packet) => {
            let task = broadcast_filters(stream, refresh_packet, db, our_peer_id, remote_peer_id);
            HandlerTaskOutput::SetOutboundRefreshTask(task.boxed())
        },
        RequestPacket::PostFilters => {
            // TODO: case if peer is already an inbound routing peer
            let config = db.get_config().await;
            let inbound_routing_state = config.in_routing_peers.state(db.in_routing_peers().await);
            match inbound_routing_state {
                MinTargetMaxState::Max | MinTargetMaxState::OverMax => HandlerTaskOutput::None,
                _ => HandlerTaskOutput::NewPendingTask(pending_get_filters(Arc::clone(&db), our_peer_id, remote_peer_id)),
            }
        }
        RequestPacket::Search(search_packet) => {
            let hashed_queries = search_packet.queries
                .iter()
                .map(|q| (q.words.iter().map(|w| D::WordHasher::hash_word(w)).collect::<Vec<_>>(), q.min_matching as usize))
                .collect::<Vec<_>>();
            let remote_matches = db.search_remote(&hashed_queries).await;

            let queries = search_packet.queries
                .iter()
                .map(|q| (q.words.to_owned(), q.min_matching as usize))
                .collect::<Vec<_>>();
            let local_matches = db.search_local(&SearchQueries::from_inner(queries)).await;

            let mut distant_matches = Vec::new();
            for (peer_id, distances) in remote_matches {
                let addresses: Vec<String> = db.get_addresses(&peer_id).await.into_iter().map(|a| a.to_string()).collect();
                if !addresses.is_empty() {
                    distant_matches.push(DistantMatch {
                        queries: distances.into_iter().map(|d| d.map(|d| d as u16)).collect(),
                        peer_id: peer_id.into(),
                        addresses,
                    });
                }
            }

            stream.start_send_unpin(ResponsePacket::Results(ResultsPacket {
                distant_matches,
                matches: local_matches.into_iter().map(|(result, query_id)| {
                    LocalMatch {
                        query: query_id as u16,
                        result: result.into_bytes(),
                    }
                }).collect()
            })).unwrap();
            stream.flush().await.unwrap();

            HandlerTaskOutput::None
        },
        RequestPacket::Disconnect(_) => todo!(),
    }
}
