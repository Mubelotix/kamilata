use crate::prelude::*;

pub struct KamilataHandlerProto {
    our_peer_id: PeerId,
    filter_db: Arc<RwLock<FilterDb>>,
}

impl IntoConnectionHandler for KamilataHandlerProto {
    type Handler = KamilataHandler;

    fn into_handler(self, remote_peer_id: &PeerId, _endpoint: &ConnectedPoint) -> Self::Handler {
        KamilataHandler::new(self.our_peer_id, remote_peer_id.to_owned(), self.filter_db)
    }

    fn inbound_protocol(&self) -> <Self::Handler as ConnectionHandler>::InboundProtocol {
        upgrade::EitherUpgrade::A(KamilataProtocolConfig::new()) // Should be KamilataHandlerConfig
    }
}

impl KamilataHandlerProto {
    pub fn new(our_peer_id: PeerId, filter_db: Arc<RwLock<FilterDb>>) -> KamilataHandlerProto {
        KamilataHandlerProto {
            our_peer_id,
            filter_db
        }
    }
}
