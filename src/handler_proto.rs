use crate::prelude::*;

pub struct KamilataHandlerProto<D: Document> {
    our_peer_id: PeerId,
    db: Arc<Db<D>>,
}

impl<D: Document> IntoConnectionHandler for KamilataHandlerProto<D> {
    type Handler = KamilataHandler<D>;

    fn into_handler(self, remote_peer_id: &PeerId, _endpoint: &ConnectedPoint) -> Self::Handler {
        KamilataHandler::new(self.our_peer_id, remote_peer_id.to_owned(), self.db)
    }

    fn inbound_protocol(&self) -> <Self::Handler as ConnectionHandler>::InboundProtocol {
        upgrade::EitherUpgrade::A(KamilataProtocolConfig::new()) // Should be KamilataHandlerConfig
    }
}

impl<D: Document> KamilataHandlerProto<D> {
    pub fn new(our_peer_id: PeerId, db: Arc<Db<D>>) -> KamilataHandlerProto<D> {
        KamilataHandlerProto {
            our_peer_id,
            db
        }
    }
}
