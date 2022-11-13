use futures::future::BoxFuture;
use libp2p::{swarm::{handler::{InboundUpgradeSend, OutboundUpgradeSend}, NegotiatedSubstream}, core::either::EitherOutput};
use std::{time::Instant, collections::HashMap};

use crate::prelude::*;

#[derive(Debug)]
pub enum KamilataHandlerIn {

}

#[derive(Debug)]
pub enum KamilataHandlerEvent {

}

type Task = BoxFuture<'static, KamTaskOutput>;

pub struct PendingTask<T> {
    params: T,
    #[allow(clippy::type_complexity)]
    fut: fn(KamOutStreamSink<NegotiatedSubstream>, T) -> Task
}

pub enum KamTaskOutput {
    None,
    Disconnect(DisconnectPacket),
    SetOutboundRefreshTask(Task)
}

async fn broadcast_local_filters<D: Document>(mut stream: KamInStreamSink<NegotiatedSubstream>, mut refresh_packet: RefreshPacket, db: Arc<Db<D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> KamTaskOutput {
    println!("{our_peer_id} Outbound refresh task executing");
    
    refresh_packet.range = refresh_packet.range.clamp(0, 10);
    refresh_packet.interval = refresh_packet.interval.clamp(15*1000, 5*60*1000); // TODO config

    stream.start_send_unpin(ResponsePacket::ConfirmRefresh(refresh_packet.clone())).unwrap();
    stream.flush().await.unwrap();

    let mut peers_to_ignore = refresh_packet.blocked_peers.as_libp2p_peer_ids();
    peers_to_ignore.push(remote_peer_id);

    loop {
        let local_filters = db.gen_local_filters(&peers_to_ignore).await;
        stream.start_send_unpin(ResponsePacket::UpdateFilters(UpdateFiltersPacket { filters: local_filters })).unwrap();
        stream.flush().await.unwrap();

        sleep(Duration::from_millis(refresh_packet.interval as u64)).await;
    }
}

async fn handle_request<D: Document>(mut stream: KamInStreamSink<NegotiatedSubstream>, filter_db: Arc<Db<D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> KamTaskOutput {
    println!("{our_peer_id} Handling a request");

    let request = stream.next().await.unwrap().unwrap();

    match request {
        RequestPacket::SetRefresh(refresh_packet) => {
            println!("{our_peer_id} It's a set refresh");
            let task = broadcast_local_filters(stream, refresh_packet, filter_db, our_peer_id, remote_peer_id);
            KamTaskOutput::SetOutboundRefreshTask(task.boxed())
        },
        RequestPacket::Search(_) => todo!(),
        RequestPacket::Disconnect(_) => todo!(),
    }
}

async fn receive_remote_filters<D: Document>(mut stream: KamOutStreamSink<NegotiatedSubstream>, db: Arc<Db<D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> KamTaskOutput {
    println!("{our_peer_id} Inbound refresh task executing");

    // Send our refresh request
    let demanded_refresh_packet = RefreshPacket::default(); // TODO: from config
    stream.start_send_unpin(RequestPacket::SetRefresh(demanded_refresh_packet.clone())).unwrap();
    stream.flush().await.unwrap();

    // Receive the response
    let response = stream.next().await.unwrap().unwrap();
    let refresh_packet = match response {
        ResponsePacket::ConfirmRefresh(refresh_packet) => refresh_packet,
        _ => return KamTaskOutput::None,
    };

    // Check response
    let blocked_peers = refresh_packet.blocked_peers.as_libp2p_peer_ids();
    let demanded_blocked_peers = demanded_refresh_packet.blocked_peers.as_libp2p_peer_ids();
    for blocked_peer in blocked_peers {
        if !demanded_blocked_peers.contains(&blocked_peer) {
            return KamTaskOutput::Disconnect(DisconnectPacket {
                reason: String::from("Could not agree on a refresh packet: blocked peer has been unblocked"),
                try_again_in: Some(86400),
            });
        }
    }

    // Receive filters
    loop {
        let packet = stream.next().await.unwrap().unwrap();
        let packet = match packet {
            ResponsePacket::UpdateFilters(packet) => packet,
            _ => return KamTaskOutput::None,
        };
        // TODO check packet.filters lenght and count and time between received
        db.set(remote_peer_id, packet.filters).await;
    }
}

fn receive_remote_filters_boxed<D: Document>(stream: KamOutStreamSink<NegotiatedSubstream>, vals: Box<dyn std::any::Any + Send>) -> Pin<Box<dyn Future<Output = KamTaskOutput> + Send>> {
    let vals: Box<(Arc<Db<D>>, PeerId, PeerId)> = vals.downcast().unwrap(); // TODO: downcast unchecked?
    receive_remote_filters(stream, vals.0, vals.1, vals.2).boxed()
}

fn pending_receive_remote_filters<D: Document>(filters: Arc<Db<D>>, our_peer_id: PeerId, remote_peer_id: PeerId) -> PendingTask<Box<dyn std::any::Any + Send>> {
    PendingTask {
        params: Box::new((filters, our_peer_id, remote_peer_id)),
        fut: receive_remote_filters_boxed::<D>
    }
}

pub struct KamilataHandler<D: Document> {
    our_peer_id: PeerId,
    remote_peer_id: PeerId,
    db: Arc<Db<D>>,

    first_poll: bool,
    rt_handle: tokio::runtime::Handle,
    
    task_counter: Counter,
    /// Tasks associated with task identifiers.  
    /// Reserved IDs:
    ///     0: outbound refresh task
    tasks: HashMap<u32, Task>,
}

impl<D: Document> KamilataHandler<D> {
    pub fn new(our_peer_id: PeerId, remote_peer_id: PeerId, db: Arc<Db<D>>) -> Self {
        let rt_handle = tokio::runtime::Handle::current();
        KamilataHandler {
            our_peer_id,
            remote_peer_id,
            db,

            first_poll: true,
            rt_handle,
            task_counter: Counter::new(1),
            tasks: HashMap::new(),
        }
    }
}

impl<D: Document> ConnectionHandler for KamilataHandler<D> {
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

    // When we receive an inbound channel, a task is immediately created to handle the channel.
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
        let task = handle_request(substream, Arc::clone(&self.db), self.our_peer_id, self.remote_peer_id).boxed();
        self.tasks.insert(self.task_counter.next(), task);
    }

    // Once an outbound is fully negotiated, the pending task which requested the establishment of the channel is now ready to be executed.
    fn inject_fully_negotiated_outbound(
        &mut self,
        substream: <Self::OutboundProtocol as OutboundUpgradeSend>::Output,
        pending_task: Self::OutboundOpenInfo,
    ) {
        let task = (pending_task.fut)(substream, pending_task.params);
        self.tasks.insert(self.task_counter.next(), task);
    }

    // Events are sent by the Behavior which we need to obey to.
    fn inject_event(&mut self, event: Self::InEvent) {
        match event {

        }
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
        // It seems this method gets called in a context where the tokio runtime does not exist.
        // We import that runtime so that we can rely on it.
        let _rt_enter_guard = self.rt_handle.enter();

        if self.first_poll {
            self.first_poll = false;

            let pending_task = pending_receive_remote_filters(Arc::clone(&self.db), self.our_peer_id, self.remote_peer_id);
            println!("{} Requesting an outbound substream for requesting inbound refreshes", self.our_peer_id);
            // TODO it is assumed that it cannot fail. Is this correct?
            return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(KamilataProtocolConfig::new(), pending_task),
            })
        }

        // Poll tasks
        for tid in self.tasks.keys().copied().collect::<Vec<u32>>() {
            let task = self.tasks.get_mut(&tid).unwrap();

            match task.poll_unpin(cx) {
                Poll::Ready(output) => {
                    println!("{} Task {tid} completed!", self.our_peer_id);
                    self.tasks.remove(&tid);

                    match output {
                        KamTaskOutput::SetOutboundRefreshTask(outbound_refresh_task) => {
                            println!("{} outbound refresh task set", self.our_peer_id);
                            self.tasks.insert(0, outbound_refresh_task);
                        },
                        KamTaskOutput::Disconnect(disconnect_packet) => {
                            println!("{} disconnected peer {}", self.our_peer_id, self.remote_peer_id);
                            // TODO: send packet
                            return Poll::Ready(ConnectionHandlerEvent::Close(
                                ioError::new(std::io::ErrorKind::Other, disconnect_packet.reason), // TODO error handling
                            ));
                        },
                        KamTaskOutput::None => (),
                    }
                }
                Poll::Pending => ()
            }
        }

        Poll::Pending // FIXME: Will this run again?
    }
}
