//! This module contains the task responsible for receiving remote filters of a peer.

use super::*;

pub(crate) async fn receive_remote_filters<const N: usize, D: Document<N>>(mut stream: KamOutStreamSink<NegotiatedSubstream>, db: Arc<Db<N, D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> HandlerTaskOutput {
    trace!("{our_peer_id} Inbound filter refresh task executing");

    // Send our request
    let config = db.get_config().await; // TODO config updates are useless
    let req = GetFiltersPacket {
        filter_count: config.filter_count as u8,
        interval: config.get_filters_interval,
        blocked_peers: Vec::new(), // TODO
    };
    stream.start_send_unpin(RequestPacket::GetFilters(req)).unwrap();
    stream.flush().await.unwrap();

    // Receive filters
    loop {
        let packet = match stream.next().await {
            Some(Ok(packet)) => packet,
            Some(Err(e)) => {
                error!("{our_peer_id} Error while receiving filters from {remote_peer_id}: {e}");
                return HandlerTaskOutput::None;
            }
            None => {
                error!("{our_peer_id} End of stream while receiving filters from {remote_peer_id}");
                return HandlerTaskOutput::None;
            }
        };
        let packet = match packet {
            ResponsePacket::UpdateFilters(packet) => packet,
            _ => {
                error!("{our_peer_id} Received unexpected packet from {remote_peer_id} while waiting for filters");
                return HandlerTaskOutput::None;
            },
        };
        // TODO check packet.filters lenght and count and time between received
        let filters = packet.filters.iter().map(|f| f.as_slice().into()).collect::<Vec<Filter<N>>>();
        db.set_remote_filter(remote_peer_id, filters).await;
        trace!("{our_peer_id} Received filters from {remote_peer_id}");
    }
}

pub(crate) fn receive_remote_filters_boxed<const N: usize, D: Document<N>>(stream: KamOutStreamSink<NegotiatedSubstream>, vals: Box<dyn std::any::Any + Send>) -> Pin<Box<dyn Future<Output = HandlerTaskOutput> + Send>> {
    let vals: Box<(Arc<Db<N, D>>, PeerId, PeerId)> = vals.downcast().unwrap(); // TODO: downcast unchecked?
    receive_remote_filters(stream, vals.0, vals.1, vals.2).boxed()
}

pub(crate) fn pending_receive_remote_filters<const N: usize, D: Document<N>>(db: Arc<Db<N, D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> PendingHandlerTask<Box<dyn std::any::Any + Send>> {
    PendingHandlerTask {
        params: Box::new((db, our_peer_id, remote_peer_id)),
        fut: receive_remote_filters_boxed::<N, D>
    }
}
