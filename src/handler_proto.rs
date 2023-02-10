use crate::prelude::*;

/// Builder for a [KamilataHandler]
pub struct KamilataHandlerBuilder<const N: usize, D: Document<N>> {
    our_peer_id: PeerId,
    db: Arc<Db<N, D>>,
}

impl<const N: usize, D: Document<N>> IntoConnectionHandler for KamilataHandlerBuilder<N, D> {
    type Handler = KamilataHandler<N, D>;

    fn into_handler(self, remote_peer_id: &PeerId, _endpoint: &ConnectedPoint) -> Self::Handler {
        KamilataHandler::new(self.our_peer_id, remote_peer_id.to_owned(), self.db)
    }

    fn inbound_protocol(&self) -> <Self::Handler as ConnectionHandler>::InboundProtocol {
        upgrade::EitherUpgrade::A(KamilataProtocolConfig::new()) // Should be KamilataHandlerConfig
    }
}

impl<const N: usize, D: Document<N>> KamilataHandlerBuilder<N, D> {
    pub(crate) fn new(our_peer_id: PeerId, db: Arc<Db<N, D>>) -> KamilataHandlerBuilder<N, D> {
        KamilataHandlerBuilder {
            our_peer_id,
            db
        }
    }
}
