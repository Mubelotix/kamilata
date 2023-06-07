//! This module contains the task responsible for broadcasting local filters to remote peers.

use super::*;

pub(crate) async fn seed_filters<const N: usize, S: Store<N>>(
    mut stream: KamInStreamSink<NegotiatedSubstream>,
    mut req: GetFiltersPacket,
    db: Arc<Db<N, S>>,
    our_peer_id: PeerId,
    remote_peer_id: PeerId
) -> HandlerTaskOutput {
    trace!("{our_peer_id} Broadcast filters task executing");
    
    // Claims a spot as a leecher for the remote peer
    if let Err(TooManyLeechers{}) = db.add_leecher(remote_peer_id).await {
        warn!("{our_peer_id} Too many leechers, can't seed to {remote_peer_id}");
        return HandlerTaskOutput::None;
    }

    // Determine an interval
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
