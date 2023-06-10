//! This module contains the task responsible for receiving remote filters of a peer.

use super::*;

// TODO: When rejecting a peer, we should send a message to the peer explaining why we rejected it

pub(crate) async fn leech_filters<const N: usize, S: Store<N>>(mut stream: KamOutStreamSink<NegotiatedSubstream>, db: Arc<Db<N, S>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> HandlerTaskOutput {
    trace!("{our_peer_id} Inbound filter refresh task executing");

    // Checks if we should allow this peer to leech
    if let Some(approve_leecher) = &db.get_config().approve_leecher {
        if !approve_leecher(remote_peer_id).await {
            warn!("{our_peer_id} {remote_peer_id} wasn't approved to leech");
            return HandlerTaskOutput::None;
        }
    }

    // Claims a spot as a seeder for the remote peer
    if let Err(TooManySeeders{}) = db.add_seeder(remote_peer_id).await {
        warn!("{our_peer_id} Too many seeders, can't leech from {remote_peer_id}");
        return HandlerTaskOutput::None;
    }

    // Send our request
    let config = db.get_config();
    let req = GetFiltersPacket {
        filter_count: config.filter_count as u8,
        interval: config.get_filters_interval.clone(),
        blocked_peers: Vec::new(), // TODO
    };
    stream.start_send_unpin(RequestPacket::GetFilters(req)).unwrap();
    stream.flush().await.unwrap();

    // Receive filters
    loop {
        let packet = match stream.next().await {
            Some(Ok(packet)) => packet,
            Some(Err(e)) => {
                warn!("{our_peer_id} Error while receiving filters from {remote_peer_id}: {e}");
                return HandlerTaskOutput::None;
            }
            None => {
                warn!("{our_peer_id} Get filters channel was closed by {remote_peer_id}");
                return HandlerTaskOutput::None;
            }
        };
        let packet = match packet {
            ResponsePacket::UpdateFilters(packet) => packet,
            _ => {
                warn!("{our_peer_id} Received unexpected packet from {remote_peer_id} while waiting for filters");
                return HandlerTaskOutput::None;
            },
        };
        // TODO check packet.filters lenght and count and time between received
        let filters = packet.filters.iter().map(|f| f.as_slice().into()).collect::<Vec<Filter<N>>>();
        db.set_remote_filter(remote_peer_id, filters).await;
        trace!("{our_peer_id} Received filters from {remote_peer_id}");
    }
}

pub(crate) fn leech_filters_boxed<const N: usize, S: Store<N>>(stream: KamOutStreamSink<NegotiatedSubstream>, vals: Box<dyn Any + Send>) -> Pin<Box<dyn Future<Output = HandlerTaskOutput> + Send>> {
    let vals: Box<(Arc<Db<N, S>>, PeerId, PeerId)> = vals.downcast().unwrap(); // TODO: downcast unchecked?
    leech_filters(stream, vals.0, vals.1, vals.2).boxed()
}

pub(crate) fn pending_leech_filters<const N: usize, S: Store<N>>(db: Arc<Db<N, S>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> PendingHandlerTask<Box<dyn Any + Send>> {
    PendingHandlerTask {
        params: Box::new((db, our_peer_id, remote_peer_id)),
        fut: leech_filters_boxed::<N, S>,
        name: "get_filters",
    }
}
