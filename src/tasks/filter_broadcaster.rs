//! This module contains the task responsible for broadcasting local filters to remote peers.

use super::*;

pub(crate) async fn broadcast_our_filters<const N: usize, D: Document<N>>(mut stream: KamInStreamSink<NegotiatedSubstream>, mut refresh_packet: RefreshPacket, db: Arc<Db<N, D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> HandlerTaskOutput {
    trace!("{our_peer_id} Outbound filter refresh task executing");
    
    refresh_packet.range = refresh_packet.range.clamp(0, 10);
    refresh_packet.interval = refresh_packet.interval.clamp(15*1000, 5*60*1000); // TODO config

    stream.start_send_unpin(ResponsePacket::ConfirmRefresh(refresh_packet.clone())).unwrap();
    stream.flush().await.unwrap();

    let mut peers_to_ignore = refresh_packet.blocked_peers.to_libp2p_peer_ids();
    peers_to_ignore.push(remote_peer_id);

    loop {
        let our_filters = db.get_filters_bytes(&peers_to_ignore).await;
        stream.start_send_unpin(ResponsePacket::UpdateFilters(UpdateFiltersPacket { filters: our_filters })).unwrap();
        stream.flush().await.unwrap();
        trace!("{our_peer_id} Sent filters to {remote_peer_id}");

        sleep(Duration::from_millis(refresh_packet.interval)).await;
    }
}
