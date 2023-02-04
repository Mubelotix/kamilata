
use crate::prelude::*;

pub enum HandlerInEvent {
    AddPendingTask(PendingHandlerTask<Box<dyn std::any::Any + Send>>),
}

impl std::fmt::Debug for HandlerInEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandlerInEvent::AddPendingTask(_) => write!(f, "AddPendingTask"),
        }
    }
}

#[derive(Debug)]
pub enum HandlerOutEvent {

}

pub struct KamilataHandler<const N: usize, D: Document<N>> {
    our_peer_id: PeerId,
    remote_peer_id: PeerId,
    db: Arc<Db<N, D>>,

    first_poll: bool,
    waker: Option<Waker>,
    rt_handle: tokio::runtime::Handle,
    
    task_counter: Counter,
    /// Tasks associated with task identifiers.  
    /// Reserved IDs:
    ///     0: outbound refresh task
    tasks: HashMap<u32, HandlerTask>,
    pending_tasks: Vec<PendingHandlerTask<Box<dyn std::any::Any + Send>>>,
}

impl<const N: usize, D: Document<N>> KamilataHandler<N, D> {
    pub fn new(our_peer_id: PeerId, remote_peer_id: PeerId, db: Arc<Db<N, D>>) -> Self {
        let rt_handle = tokio::runtime::Handle::current();
        KamilataHandler {
            our_peer_id,
            remote_peer_id,
            db,

            first_poll: true,
            waker: None,
            rt_handle,
            task_counter: Counter::new(1),
            tasks: HashMap::new(),
            pending_tasks: Vec::new(),
        }
    }
}

impl<const N: usize, D: Document<N>> ConnectionHandler for KamilataHandler<N, D> {
    type InEvent = HandlerInEvent;
    type OutEvent = HandlerOutEvent;
    type Error = ioError;
    type InboundProtocol = EitherUpgrade<KamilataProtocolConfig, DeniedUpgrade>;
    type OutboundProtocol = KamilataProtocolConfig;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = PendingHandlerTask<Box<dyn std::any::Any + Send>>;

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
        println!("{} Established outbound channel with {}", self.our_peer_id, self.remote_peer_id);
        let task = (pending_task.fut)(substream, pending_task.params);
        self.tasks.insert(self.task_counter.next(), task);
    }

    // Events are sent by the Behavior which we need to obey to.
    fn inject_event(&mut self, event: Self::InEvent) {
        match event {
            HandlerInEvent::AddPendingTask(pending_task) => {
                self.pending_tasks.push(pending_task);
                if let Some(waker) = self.waker.take() {
                    waker.wake();
                }
            },
        };
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
            self.pending_tasks.push(pending_task);
        }

        if let Some(pending_task) = self.pending_tasks.pop() {
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
                        HandlerTaskOutput::SetOutboundRefreshTask(mut outbound_refresh_task) => {
                            println!("{} outbound refresh task set", self.our_peer_id);
                            outbound_refresh_task.poll_unpin(cx); // TODO should be used
                            self.tasks.insert(0, outbound_refresh_task);
                        },
                        HandlerTaskOutput::Disconnect(disconnect_packet) => {
                            println!("{} disconnected peer {}", self.our_peer_id, self.remote_peer_id);
                            // TODO: send packet
                            return Poll::Ready(ConnectionHandlerEvent::Close(
                                ioError::new(std::io::ErrorKind::Other, disconnect_packet.reason), // TODO error handling
                            ));
                        },
                        HandlerTaskOutput::None => (),
                    }
                }
                Poll::Pending => ()
            }
        }

        Poll::Pending // FIXME: Will this run again?
    }
}
