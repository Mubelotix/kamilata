//! This module contains the task responsible for broadcasting local filters to remote peers.

use super::*;

pub(crate) async fn broadcast_filters<const N: usize, D: Document<N>>(mut stream: KamInStreamSink<NegotiatedSubstream>, mut req: GetFiltersPacket, db: Arc<Db<N, D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> HandlerTaskOutput {
    trace!("{our_peer_id} Outbound filter refresh task executing");
    
    let config = db.get_config().await;
    req.filter_count = req.filter_count.clamp(0, config.filter_count as u8); // unsafe cast
    let interval = match config.get_filters_interval.intersection(&req.interval) {
        Some(interval) => interval.target() as u64,
        None => {
            warn!("{our_peer_id} Couldn't agree on interval with {remote_peer_id} (ours: {:?}, theirs: {:?})", config.get_filters_interval, req.interval);
            return HandlerTaskOutput::None;
        }
    };

    let mut peers_to_ignore = req.blocked_peers.to_libp2p_peer_ids();
    peers_to_ignore.push(remote_peer_id);

    loop {
        let our_filters = db.get_filters_bytes(&peers_to_ignore).await;
        stream.start_send_unpin(ResponsePacket::UpdateFilters(UpdateFiltersPacket { filters: our_filters })).unwrap();
        stream.flush().await.unwrap();
        trace!("{our_peer_id} Sent filters to {remote_peer_id}");

        sleep(Duration::from_millis(interval)).await;
    }
}

pub(crate) async fn post_filters(mut stream: KamOutStreamSink<NegotiatedSubstream>, our_peer_id: PeerId, remote_peer_id: PeerId) -> HandlerTaskOutput {
    trace!("{our_peer_id} Telling {remote_peer_id} we would like to broadcast our filters");

    stream.start_send_unpin(RequestPacket::PostFilters).unwrap();
    stream.flush().await.unwrap();

    HandlerTaskOutput::None
}

pub(crate) fn post_filters_boxed(stream: KamOutStreamSink<NegotiatedSubstream>, vals: Box<dyn std::any::Any + Send>) -> Pin<Box<dyn Future<Output = HandlerTaskOutput> + Send>> {
    let vals: Box<(PeerId, PeerId)> = vals.downcast().unwrap();
    post_filters(stream, vals.0, vals.1).boxed()
}

pub(crate) fn pending_post_filters(our_peer_id: PeerId, remote_peer_id: PeerId) -> PendingHandlerTask<Box<dyn std::any::Any + Send>> {
    PendingHandlerTask {
        params: Box::new((our_peer_id, remote_peer_id)),
        fut: post_filters_boxed,
        name: "post_filters",
    }
}
