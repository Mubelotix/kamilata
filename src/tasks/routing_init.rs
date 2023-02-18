use super::*;
use MinTargetMaxState::*;

pub(crate) async fn init_routing<const N: usize, D: Document<N>>(db: Arc<Db<N, D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> HandlerTaskOutput {
    let config = db.get_config().await;

    let in_routing_state = config.in_routing_peers.state(db.in_routing_peers().await);
    let mut pending_in_routing_task = None;
    if matches!(in_routing_state, UnderMin | Min | UnderTarget) {
        pending_in_routing_task = Some(pending_get_filters(Arc::clone(&db), our_peer_id, remote_peer_id));
    }

    let out_routing_state = config.out_routing_peers.state(db.out_routing_peers().await);
    let mut pending_out_routing_task = None;
    if matches!(out_routing_state, UnderMin | Min | UnderTarget) {
        pending_out_routing_task = Some(pending_post_filters(our_peer_id, remote_peer_id));
    }
    
    match (pending_in_routing_task, pending_out_routing_task) {
        (Some(t1), Some(t2)) => HandlerTaskOutput::Many(vec![HandlerTaskOutput::NewPendingTask(t1), HandlerTaskOutput::NewPendingTask(t2)]),
        (Some(t), None) | (None, Some(t)) => HandlerTaskOutput::NewPendingTask(t),
        (None, None) => HandlerTaskOutput::None
    }
}
