use super::*;

pub async fn handle_request<const N: usize, D: Document<N>>(mut stream: KamInStreamSink<NegotiatedSubstream>, filter_db: Arc<Db<N, D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> HandlerTaskOutput {
    println!("{our_peer_id} Handling a request");

    let request = stream.next().await.unwrap().unwrap();

    match request {
        RequestPacket::SetRefresh(refresh_packet) => {
            println!("{our_peer_id} It's a set refresh");
            let task = broadcast_local_filters(stream, refresh_packet, filter_db, our_peer_id, remote_peer_id);
            HandlerTaskOutput::SetOutboundRefreshTask(task.boxed())
        },
        RequestPacket::Search(search_packet) => {
            println!("{our_peer_id} It's a search");
            let hashed_queries = search_packet.queries
                .iter()
                .map(|q| (q.words.iter().map(|w| D::WordHasher::hash_word(w)).collect::<Vec<_>>(), q.min_matching as usize))
                .collect::<Vec<_>>();
            let remote_matches = filter_db.search_remote(&hashed_queries).await;

            let queries = search_packet.queries
                .iter()
                .map(|q| (q.words.to_owned(), q.min_matching as usize))
                .collect::<Vec<_>>();
            let local_matches = filter_db.search_local(&queries).await;

            stream.start_send_unpin(ResponsePacket::Results(ResultsPacket {
                routes: remote_matches.into_iter().map(|(peer_id, distances)| {
                    RemoteMatch {
                        queries: distances.into_iter().map(|d| d.map(|d| d as u16)).collect(),
                        peer_id: peer_id.into(),
                    }
                }).collect(),
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
