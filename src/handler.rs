use crate::prelude::*;

#[derive(Debug)]
pub enum KamilataHandlerIn {

}

#[derive(Debug)]
pub enum KamilataHandlerEvent {

}

pub struct KamilataHandler {
}

impl KamilataHandler {
    pub fn new() -> Self {
        KamilataHandler {

        }
    }
}

impl ConnectionHandler for KamilataHandler {
    type InEvent = KamilataHandlerIn;
    type OutEvent = KamilataHandlerEvent;
    type Error = std::io::Error;
    type InboundProtocol = EitherUpgrade<KamilataProtocolConfig, DeniedUpgrade>;
    type OutboundProtocol = KamilataProtocolConfig;
    type InboundOpenInfo = (); // TODO we might need things here
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(KamilataProtocolConfig::new(), ()).map_upgrade(upgrade::EitherUpgrade::A)
        // todo What is that () ?
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        protocol: <Self::InboundProtocol as libp2p::swarm::handler::InboundUpgradeSend>::Output,
        info: Self::InboundOpenInfo,
    ) {
        // We should probably store an outbound_stream 
        todo!()
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        protocol: <Self::OutboundProtocol as libp2p::swarm::handler::OutboundUpgradeSend>::Output,
        info: Self::OutboundOpenInfo,
    ) {
        todo!()
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        todo!()
    }

    fn inject_dial_upgrade_error(
        &mut self,
        info: Self::OutboundOpenInfo,
        error: libp2p::swarm::ConnectionHandlerUpgrErr<<Self::OutboundProtocol as libp2p::swarm::handler::OutboundUpgradeSend>::Error>,
    ) {
        todo!()
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        KeepAlive::Yes
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<
        ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::OutEvent,
            Self::Error,
        >,
    > {
        Poll::Pending
    }
}

pub struct KamilataHandlerProto {

}

impl IntoConnectionHandler for KamilataHandlerProto {
    type Handler = KamilataHandler;

    fn into_handler(self, remote_peer_id: &PeerId, endpoint: &ConnectedPoint) -> Self::Handler {
        KamilataHandler::new()
    }

    fn inbound_protocol(&self) -> <Self::Handler as ConnectionHandler>::InboundProtocol {
        upgrade::EitherUpgrade::A(KamilataProtocolConfig::new()) // Should be KamilataHandlerConfig
    }
}

impl KamilataHandlerProto {
    pub fn new() -> KamilataHandlerProto {
        KamilataHandlerProto {}
    }
}