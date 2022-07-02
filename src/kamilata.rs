
use crate::prelude::*;

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

pub enum KamilataHandshakeError {

}

impl<TSocket> InboundUpgrade<TSocket> for KamilataProtocolConfig
where
    TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = TSocket;
    type Error = KamilataHandshakeError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, socket: TSocket, _: Self::Info) -> Self::Future {
        Box::pin(async move {
            Ok(socket)
        })
    }
}

impl<TSocket> OutboundUpgrade<TSocket> for KamilataProtocolConfig
where
    TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = TSocket;
    type Error = KamilataHandshakeError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, socket: TSocket, _: Self::Info) -> Self::Future {
        Box::pin(async move {
            Ok(socket)
        })
    }
}


#[derive(Debug)]
pub enum KamilataEvent {

}

pub struct Kamilata {

}

impl NetworkBehaviour for Kamilata {
    type ConnectionHandler = KamilataHandlerProto;
    type OutEvent = KamilataEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        KamilataHandlerProto::new()
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

impl Kamilata {
    pub fn new() -> Kamilata {
        Kamilata {
            
        }
    }
}
