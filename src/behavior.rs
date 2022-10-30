use crate::prelude::*;

#[derive(Debug)]
pub enum KamilataEvent {

}

pub struct KamilataBehavior {
    our_peer_id: PeerId,
}

impl NetworkBehaviour for KamilataBehavior {
    type ConnectionHandler = KamilataHandlerProto;
    type OutEvent = KamilataEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        KamilataHandlerProto::new(self.our_peer_id)
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

impl KamilataBehavior {
    pub fn new(our_peer_id: PeerId) -> KamilataBehavior {
        KamilataBehavior {
            our_peer_id,
        }
    }
}
