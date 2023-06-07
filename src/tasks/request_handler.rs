//! This module contains the handler of requests from remote peers.

use super::*;

pub(crate) async fn handle_request<const N: usize, S: Store<N>>(
    mut stream: KamInStreamSink<NegotiatedSubstream>,
    db: Arc<Db<N, S>>,
    our_peer_id: PeerId,
    remote_peer_id: PeerId
) -> HandlerTaskOutput {
    let request = match stream.next().await {
        Some(Ok(request)) => request,
        Some(Err(e)) => {
            error!("{our_peer_id} Error while receiving request from {remote_peer_id}: {e}");
            return HandlerTaskOutput::None;
        },
        None => return HandlerTaskOutput::None,
    };

    debug!("{our_peer_id} Request from {remote_peer_id}: {request:?}");
    match request {
        RequestPacket::GetFilters(refresh_packet) => {
            let task = seed_filters(stream, refresh_packet, db, our_peer_id, remote_peer_id);
            HandlerTaskOutput::SetTask {
                tid: 1,
                task: HandlerTask { fut: Box::pin(task), name: "broadcast_filters" },
            }
        },
        RequestPacket::Search(search_packet) => {
            let hashed_queries = search_packet.queries
                .iter()
                .map(|q| (q.words.iter().map(|w| S::hash_word(w)).collect::<Vec<_>>(), q.min_matching as usize))
                .collect::<Vec<_>>();
            let remote_matches = db.search_remote(&hashed_queries).await;

            let queries = search_packet.queries
                .iter()
                .map(|q| (q.words.to_owned(), q.min_matching as usize))
                .collect::<Vec<_>>();
            let local_matches = join_all(queries.into_iter().map(|(words, min_matching)| db.store().search(words, min_matching))).await;

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

            let mut matches = Vec::new();
            for (query_id, results) in local_matches.into_iter().enumerate() {
                for result in results {
                    matches.push(LocalMatch {
                        query: query_id as u16,
                        result: result.into_bytes(),
                    })
                }
            }
            stream.start_send_unpin(ResponsePacket::Results(ResultsPacket {
                distant_matches,
                matches
            })).unwrap();
            stream.flush().await.unwrap();
            trace!("{our_peer_id} Sent the data to {remote_peer_id}.");

            HandlerTaskOutput::None
        },
        RequestPacket::Disconnect(_) => todo!(),
    }
}
