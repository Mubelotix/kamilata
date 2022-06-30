use std::{iter, pin::Pin};
use futures::{Future, prelude::*};
use libp2p::{swarm::{IntoConnectionHandler, ConnectionHandler, SubstreamProtocol}, core::{ConnectedPoint, upgrade::{EitherUpgrade, DeniedUpgrade, self}, UpgradeInfo}, InboundUpgrade, kad::protocol::{KadInStreamSink, KadOutStreamSink}, OutboundUpgrade, PeerId};

#[derive(Debug, Clone, Default)]
pub struct KamilataProtocolConfig {}

impl KamilataProtocolConfig {
    pub fn new() -> KamilataProtocolConfig {
        KamilataProtocolConfig {}
    }
}

impl UpgradeInfo for KamilataProtocolConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/kamilata/0.0.1")
    }
}

/// TODO: no idea what this will do
pub struct KamilataHandshakeOutput {

}
pub enum KamilataHandshakeError {

}

impl<TSocket> InboundUpgrade<TSocket> for KamilataProtocolConfig
where
    TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = KamilataHandshakeOutput;
    type Error = KamilataHandshakeError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, mut socket: TSocket, _: Self::Info) -> Self::Future {
        Box::pin(async move {
            // TODO see floodsub
            Ok(KamilataHandshakeOutput {

            })
        })
    }
}

impl<TSocket> OutboundUpgrade<TSocket> for KamilataProtocolConfig
where
    TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = KamilataHandshakeOutput;
    type Error = KamilataHandshakeError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, mut socket: TSocket, _: Self::Info) -> Self::Future {
        Box::pin(async move {
            // TODO see floodsub
            Ok(KamilataHandshakeOutput {

            })
        })
    }
}

#[derive(Debug)]
enum KamilataHandlerIn {

}

#[derive(Debug)]
enum KamilataHandlerEvent {

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
        // This is where we receive shit
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
