use std::task::Waker;
use crate::prelude::*;

/// Events aimed at a [KamilataHandler]
pub enum HandlerInEvent {
    /// Asks the handler to send a request and receive a response.
    Request {
        /// This request packet will be sent through a new outbound substream.
        request: RequestPacket,
        /// The response will be sent back through this channel.
        sender: OneshotSender<Option<ResponsePacket>>,
    },
}

impl std::fmt::Debug for HandlerInEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandlerInEvent::Request { .. } => write!(f, "Request"),
        }
    }
}

/// Events produced by a [KamilataHandler] (unused)
#[derive(Debug)]
pub enum HandlerOutEvent {}

/// The [KamilataHandler] is responsible for handling a connection to a remote peer.
/// Multiple handlers are managed by the [KamilataBehavior].
pub struct KamilataHandler<const N: usize, D: Document<N>> {
    our_peer_id: PeerId,
    remote_peer_id: PeerId,
    db: Arc<Db<N, D>>,

    rt_handle: tokio::runtime::Handle,
    
    task_counter: Counter,
    /// Tasks associated with task identifiers.  
    /// Reserved IDs:
    ///     0: routing initialization
    ///     1: filter broadcaster
    ///     2: filter receiver
    tasks: HashMap<u32, HandlerTask>,
    /// Tasks waiting to be inserted into the `tasks` map, because their outbound substream is still opening.
    pending_tasks: Vec<(Option<(u32, bool)>, PendingHandlerTask<Box<dyn std::any::Any + Send>>)>,
}

impl<const N: usize, D: Document<N>> KamilataHandler<N, D> {
    pub(crate) fn new(our_peer_id: PeerId, remote_peer_id: PeerId, db: Arc<Db<N, D>>) -> Self {
        let rt_handle = tokio::runtime::Handle::current();
        let task_counter = Counter::new(3);
        let mut tasks: HashMap<u32, HandlerTask> = HashMap::new();
        let pending_tasks = Vec::new();

        let init_routing_fut = init_routing(Arc::clone(&db), our_peer_id, remote_peer_id);
        tasks.insert(0, HandlerTask { fut: Box::pin(init_routing_fut), name: "init_routing" });

        KamilataHandler {
            our_peer_id, remote_peer_id, db, rt_handle, task_counter, tasks, pending_tasks
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
    type OutboundOpenInfo = (Option<(u32, bool)>, PendingHandlerTask<Box<dyn std::any::Any + Send>>);

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
        let fut = handle_request(substream, Arc::clone(&self.db), self.our_peer_id, self.remote_peer_id).boxed();
        self.tasks.insert(self.task_counter.next(), HandlerTask { fut, name: "handle_request" });
    }

    // Once an outbound is fully negotiated, the pending task which requested the establishment of the channel is now ready to be executed.
    fn inject_fully_negotiated_outbound(
        &mut self,
        substream: <Self::OutboundProtocol as OutboundUpgradeSend>::Output,
        (tid, pending_task): Self::OutboundOpenInfo,
    ) {
        let fut = (pending_task.fut)(substream, pending_task.params);
        let (tid, replace) = tid.unwrap_or_else(|| (self.task_counter.next(), true));
        if self.tasks.contains_key(&tid) && !replace {
            return;
        }
        if let Some(old_task) = self.tasks.insert(tid, HandlerTask { fut, name: pending_task.name }) {
            warn!("{} Replaced {} task with {} task at tid={tid}", self.our_peer_id, old_task.name, pending_task.name)
        }
    }

    // Events are sent by the Behavior which we need to obey to.
    fn inject_event(&mut self, event: Self::InEvent) {
        match event {
            HandlerInEvent::Request { request, sender } => {
                let pending_task = pending_request::<N, D>(request, sender, self.our_peer_id, self.remote_peer_id);
                self.pending_tasks.push((None, pending_task));
            },
        };
    }

    fn inject_dial_upgrade_error(
        &mut self,
        (_tid, pending_task): Self::OutboundOpenInfo,
        error: libp2p::swarm::ConnectionHandlerUpgrErr<<Self::OutboundProtocol as OutboundUpgradeSend>::Error>,
    ) {
        warn!("{} Failed to establish outbound channel with {}: {error:?}. A {} task has been discarded.", self.our_peer_id, self.remote_peer_id, pending_task.name);
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

        // Poll tasks
        for tid in self.tasks.keys().copied().collect::<Vec<u32>>() {
            let task = self.tasks.get_mut(&tid).unwrap();

            match task.fut.poll_unpin(cx) {
                Poll::Ready(output) => {
                    trace!("{} Task {} completed (tid={tid})", self.our_peer_id, task.name);
                    self.tasks.remove(&tid);

                    for output in output.into_vec() {
                        match output {
                            HandlerTaskOutput::SetTask { tid, mut task } => {
                                match self.tasks.get(&tid) {
                                    Some(old_task) => warn!("{} Replacing {} task with {} task at tid={tid}", self.our_peer_id, old_task.name, task.name),
                                    None => trace!("{} Inserting {} task at tid={tid}", self.our_peer_id, task.name)                                    ,
                                }
                                task.fut.poll_unpin(cx); // TODO should be used
                                self.tasks.insert(tid, task);
                            },
                            HandlerTaskOutput::NewPendingTask { tid, pending_task } => {
                                trace!("{} New pending task: {}", self.our_peer_id, pending_task.name);
                                self.pending_tasks.push((tid, pending_task));
                            },
                            HandlerTaskOutput::Disconnect(disconnect_packet) => {
                                debug!("{} Disconnected peer {}", self.our_peer_id, self.remote_peer_id);
                                // TODO: send packet
                                return Poll::Ready(ConnectionHandlerEvent::Close(
                                    ioError::new(std::io::ErrorKind::Other, disconnect_packet.reason), // TODO error handling
                                ));
                            },
                            HandlerTaskOutput::None | HandlerTaskOutput::Many(_) => unreachable!(),
                        }
                    }
                }
                Poll::Pending => ()
            }
        }   

        if let Some((tid, pending_task)) = self.pending_tasks.pop() {
            return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(KamilataProtocolConfig::new(), (tid, pending_task)),
            })
        }

        // It seems we don't have to care about waking up the handler because libp2p does it when inject methods are called.
        // A link to documentation would be appreciated.
        Poll::Pending
    }
}
