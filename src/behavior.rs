use crate::prelude::*;

#[derive(Debug)]
pub enum KamilataEvent {

}

pub struct KamilataBehavior<D: Document> {
    our_peer_id: PeerId,
    db: Arc<Db<D>>
}

impl<D: Document> NetworkBehaviour for KamilataBehavior<D> {
    type ConnectionHandler = KamilataHandlerProto<D>;
    type OutEvent = KamilataEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        KamilataHandlerProto::new(self.our_peer_id, Arc::clone(&self.db))
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: <<Self::ConnectionHandler as IntoConnectionHandler>::Handler as ConnectionHandler>::OutEvent,
    ) {
        todo!()
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        Poll::Pending
    }
}

impl<D: Document> KamilataBehavior<D> {
    pub fn new(our_peer_id: PeerId) -> KamilataBehavior<D> {
        KamilataBehavior {
            our_peer_id,
            db: Arc::new(Db::new())
        }
    }
}
