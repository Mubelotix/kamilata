use super::*;

pub async fn receive_remote_filters<const N: usize, D: Document<N>>(mut stream: KamOutStreamSink<NegotiatedSubstream>, db: Arc<Db<N, D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> HandlerTaskOutput {
    println!("{our_peer_id} Inbound refresh task executing");

    // Send our refresh request
    let demanded_refresh_packet = RefreshPacket::default(); // TODO: from config
    stream.start_send_unpin(RequestPacket::SetRefresh(demanded_refresh_packet.clone())).unwrap();
    stream.flush().await.unwrap();

    // Receive the response
    let response = stream.next().await.unwrap().unwrap();
    let refresh_packet = match response {
        ResponsePacket::ConfirmRefresh(refresh_packet) => refresh_packet,
        _ => {
            println!("{our_peer_id} Received unexpected packet from {remote_peer_id} while waiting for refresh confirmation packet");
            return HandlerTaskOutput::None;
        },
    };

    // Check response
    let blocked_peers = refresh_packet.blocked_peers.as_libp2p_peer_ids();
    let demanded_blocked_peers = demanded_refresh_packet.blocked_peers.as_libp2p_peer_ids();
    for blocked_peer in blocked_peers {
        if !demanded_blocked_peers.contains(&blocked_peer) {
            return HandlerTaskOutput::Disconnect(DisconnectPacket {
                reason: String::from("Could not agree on a refresh packet: blocked peer has been unblocked"),
                try_again_in: Some(86400),
            });
        }
    }

    // Receive filters
    loop {
        let packet = stream.next().await.unwrap().unwrap();
        let packet = match packet {
            ResponsePacket::UpdateFilters(packet) => packet,
            _ => {
                println!("{our_peer_id} Received unexpected packet from {remote_peer_id} while waiting for filters");
                return HandlerTaskOutput::None;
            },
        };
        // TODO check packet.filters lenght and count and time between received
        let filters = packet.filters.iter().map(|f| f.as_slice().into()).collect::<Vec<Filter<N>>>();
        let load = filters.first().map(|f| f.load()).unwrap_or(0.0);
        db.set_remote_filter(remote_peer_id, filters).await;
        println!("{our_peer_id} Received filters from {remote_peer_id} (load: {load})");
    }
}

pub fn receive_remote_filters_boxed<const N: usize, D: Document<N>>(stream: KamOutStreamSink<NegotiatedSubstream>, vals: Box<dyn std::any::Any + Send>) -> Pin<Box<dyn Future<Output = HandlerTaskOutput> + Send>> {
    let vals: Box<(Arc<Db<N, D>>, PeerId, PeerId)> = vals.downcast().unwrap(); // TODO: downcast unchecked?
    receive_remote_filters(stream, vals.0, vals.1, vals.2).boxed()
}

pub fn pending_receive_remote_filters<const N: usize, D: Document<N>>(filters: Arc<Db<N, D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> PendingHandlerTask<Box<dyn std::any::Any + Send>> {
    PendingHandlerTask {
        params: Box::new((filters, our_peer_id, remote_peer_id)),
        fut: receive_remote_filters_boxed::<N, D>
    }
}
