use libp2p::{swarm::{handler::{InboundUpgradeSend, OutboundUpgradeSend}, NegotiatedSubstream}, core::either::EitherOutput};
use std::{time::Instant, collections::HashMap};

use crate::prelude::*;

#[derive(Debug)]
pub enum KamilataHandlerIn {

}

#[derive(Debug)]
pub enum KamilataHandlerEvent {

}

type Task = Pin<Box<dyn Future<Output = KamTaskOutput> + Send>>;

pub struct PendingTask<T> {
    params: T,
    #[allow(clippy::type_complexity)]
    fut: fn(KamOutStreamSink<NegotiatedSubstream>, T) -> Task
}

pub enum KamTaskOutput {
    SetOutboundRefreshTask(Task)
}

pub struct KamilataHandler {
    first_poll: bool,
    task_counter: Counter,
    /// Tasks associated with task identifiers.  
    /// Reserved IDs:
    ///     0: outbound refresh task
    tasks: HashMap<u32, Task>,
}

impl KamilataHandler {
    pub fn new() -> Self {
        KamilataHandler {
            first_poll: true,
            task_counter: Counter::new(1),
            tasks: HashMap::new(),
        }
    }
}

async fn outbound_refresh(mut stream: KamInStreamSink<NegotiatedSubstream>, mut refresh_packet: RefreshPacket) -> KamTaskOutput {
    refresh_packet.range = refresh_packet.range.clamp(0, 10);
    refresh_packet.interval = refresh_packet.interval.clamp(15*1000, 5*60*1000); // TODO config

    stream.start_send_unpin(ResponsePacket::ConfirmRefresh(refresh_packet.clone())).unwrap();
    stream.flush().await.unwrap();

    loop {
        // TODO: send actual packets

        sleep(Duration::from_millis(refresh_packet.interval as u64)).await;
    }
}

async fn handle_request(mut stream: KamInStreamSink<NegotiatedSubstream>) -> KamTaskOutput {
    let request = stream.next().await.unwrap().unwrap();

    match request {
        RequestPacket::SetRefresh(refresh_packet) => {
            let task = outbound_refresh(stream, refresh_packet);
            KamTaskOutput::SetOutboundRefreshTask(task.boxed())
        },
        RequestPacket::FindPeers(_) => todo!(),
        RequestPacket::Search(_) => todo!(),
        RequestPacket::RewardPeer(_) => todo!(),
        RequestPacket::Disconnect(_) => todo!(),
    }
}

async fn inbound_refresh(stream: KamOutStreamSink<NegotiatedSubstream>) -> KamTaskOutput {
    todo!()
}

fn inbound_refresh_boxed(stream: KamOutStreamSink<NegotiatedSubstream>, _val: Box<dyn std::any::Any + Send>) -> Pin<Box<dyn Future<Output = KamTaskOutput> + Send>> {
    inbound_refresh(stream).boxed()
}

fn pending_inbound_refresh() -> PendingTask<Box<dyn std::any::Any + Send>> {
    PendingTask {
        params: Box::new(()),
        fut: inbound_refresh_boxed
    }
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
        let task = handle_request(substream).boxed();
        self.tasks.insert(self.task_counter.next(), task);
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        substream: <Self::OutboundProtocol as OutboundUpgradeSend>::Output,
        pending_task: Self::OutboundOpenInfo,
    ) {
        let task = (pending_task.fut)(substream, pending_task.params);
        self.tasks.insert(self.task_counter.next(), task);
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
        if self.first_poll {
            self.first_poll = false;

            let pending_task = pending_inbound_refresh();
            // TODO it is assumed that it cannot fail. Is this correct?
            return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(KamilataProtocolConfig::new(), pending_task),
            })
        }

        // Poll tasks
        for tid in self.tasks.keys().copied().collect::<Vec<u32>>() {
            let task = self.tasks.get_mut(&tid).unwrap();

            match task.poll_unpin(cx) {
                Poll::Ready(output) => match output {
                    KamTaskOutput::SetOutboundRefreshTask(outbound_refresh_task) => {
                        self.tasks.insert(0, outbound_refresh_task);
                    },
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