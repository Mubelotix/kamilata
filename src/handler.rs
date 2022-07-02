use std::sync::Arc;
use future::FutureExt;
use libp2p::{swarm::{handler::{InboundUpgradeSend, OutboundUpgradeSend}, NegotiatedSubstream}, core::either::EitherOutput};

use crate::prelude::*;

#[derive(Debug)]
pub enum KamilataHandlerIn {

}

#[derive(Debug)]
pub enum KamilataHandlerEvent {

}

pub enum KamTaskOutput {

}

pub struct KamilataHandler {
    tasks: Vec<Pin<Box<dyn Future<Output = KamTaskOutput> + Send>>>,
}

impl KamilataHandler {
    pub fn new() -> Self {
        KamilataHandler {
            tasks: Vec::new(),
        }
    }
}

async fn foo(stream: KamInStreamSink<NegotiatedSubstream>) -> KamTaskOutput {
    todo!()
}

pub struct PendingTask<T> {
    params: T,
    fut: fn(KamOutStreamSink<NegotiatedSubstream>, T) -> Pin<Box<dyn Future<Output = KamTaskOutput> + Send>>
}

impl ConnectionHandler for KamilataHandler {
    type InEvent = KamilataHandlerIn;
    type OutEvent = KamilataHandlerEvent;
    type Error = ioError;
    type InboundProtocol = EitherUpgrade<KamilataProtocolConfig, DeniedUpgrade>;
    type OutboundProtocol = KamilataProtocolConfig;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = PendingTask<Box<dyn std::any::Any + Send>>;

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(KamilataProtocolConfig::new(), ()).map_upgrade(upgrade::EitherUpgrade::A)
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        protocol: <Self::InboundProtocol as InboundUpgradeSend>::Output,
        _: Self::InboundOpenInfo,
    ) {
        let substream = match protocol {
            EitherOutput::First(s) => s,
            EitherOutput::Second(_void) => return,
        };

        // TODO: prevent DoS

        self.tasks.push(Box::pin(foo(substream)))
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        substream: <Self::OutboundProtocol as OutboundUpgradeSend>::Output,
        pending_task: Self::OutboundOpenInfo,
    ) {
        let task = (pending_task.fut)(substream, pending_task.params);
        self.tasks.push(task);
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        todo!()
    }

    fn inject_dial_upgrade_error(
        &mut self,
        info: Self::OutboundOpenInfo,
        error: libp2p::swarm::ConnectionHandlerUpgrErr<<Self::OutboundProtocol as OutboundUpgradeSend>::Error>,
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
        // Poll tasks
        for task in &mut self.tasks {
            match task.poll_unpin(cx) {
                Poll::Ready(output) => match output {
                    
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

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