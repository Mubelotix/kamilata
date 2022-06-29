use std::pin::Pin;

use libp2p::{swarm::{IntoConnectionHandler, ConnectionHandler}, core::{ConnectedPoint, upgrade::{EitherUpgrade, DeniedUpgrade}, UpgradeInfo}, InboundUpgrade, kad::protocol::{KadInStreamSink, KadOutStreamSink}, OutboundUpgrade};

use crate::prelude::*;

#[derive(Debug)]
enum KamilataHandlerIn {

}

#[derive(Debug)]
enum KamilataHandlerEvent {

}

struct KamilataProtocolConfig {
    // protocol name
    // max packet size
}

impl UpgradeInfo for KamilataProtocolConfig {
    type Info = &'static [u8];
    type InfoIter = std::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        std::iter::once(b"/kamilata/0.0.0")
    }
}

async fn handshake<C: AsyncRead + AsyncWrite + Unpin>(socket: C) -> Result<KadInStreamSink<C>, std::io::Error> {
    todo!()
}

impl<C> InboundUpgrade<C> for KamilataProtocolConfig
where
    C: AsyncRead + AsyncWrite + Unpin,
{
    type Output = KadInStreamSink<C>; // TODO remove Kademlia ref
    type Error = std::io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, socket: C, info: Self::Info) -> Self::Future {
        Box::pin(handshake(socket))
    }
}

impl<C> OutboundUpgrade<C> for KamilataProtocolConfig
where
    C: AsyncRead + AsyncWrite + Unpin,
{
    type Output = KadOutStreamSink<C>; // TODO remove Kademlia ref
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, socket: C, info: Self::Info) -> Self::Future {
        todo!()
    }
}

struct KamilataHandler {

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
    type InboundOpenInfo = KamilataHandlerIn; // TODO should be different
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        todo!()
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        protocol: <Self::InboundProtocol as libp2p::swarm::handler::InboundUpgradeSend>::Output,
        info: Self::InboundOpenInfo,
    ) {
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

    fn connection_keep_alive(&self) -> libp2p::swarm::KeepAlive {
        todo!()
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        libp2p::swarm::ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::OutEvent,
            Self::Error,
        >,
    > {
        todo!()
    }
}

struct KamilataHandlerProto {
}

impl IntoConnectionHandler for KamilataHandlerProto {
    type Handler = KamilataHandler;

    fn into_handler(self, remote_peer_id: &PeerId, endpoint: &ConnectedPoint) -> Self::Handler {
        KamilataHandler::new()
    }

    fn inbound_protocol(&self) -> <Self::Handler as ConnectionHandler>::InboundProtocol {
        todo!()
    }
}

enum KamilataEvent {

}

struct Kamilata {

}

impl NetworkBehaviour for Kamilata {
    type ConnectionHandler = KamilataHandlerProto;
    type OutEvent = KamilataEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        todo!()
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: libp2p::core::connection::ConnectionId,
        event: <<Self::ConnectionHandler as libp2p::swarm::IntoConnectionHandler>::Handler as libp2p::swarm::ConnectionHandler>::OutEvent,
    ) {
        todo!()
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
        params: &mut impl libp2p::swarm::PollParameters,
    ) -> std::task::Poll<libp2p::swarm::NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        todo!()
    }
}
