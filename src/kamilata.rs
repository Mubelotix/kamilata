use std::{iter, pin::Pin};
use futures::Future;
use libp2p::{swarm::{IntoConnectionHandler, ConnectionHandler}, core::{ConnectedPoint, upgrade::{EitherUpgrade, DeniedUpgrade}, UpgradeInfo}, InboundUpgrade, kad::protocol::{KadInStreamSink, KadOutStreamSink}, OutboundUpgrade};
use tokio::io::{AsyncRead, AsyncWrite};

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


// Could implement UpgradeInfo and OutboundUpgrade on KamilataHandshakeOutput but no idea why
