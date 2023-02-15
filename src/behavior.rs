use crate::prelude::*;

/// Events produced by the [KamilataBehavior]
#[derive(Debug)]
pub enum KamilataEvent {
    // TODO unroutable, routable and pending
    UnroutablePeer(PeerId),
}

/// Implementation of the Kamilata protocol.
/// 
/// # Peer Discovery
/// 
/// The [KamilataBehavior] does not provide peer discovery by itself.
/// Peer discovery is the process by which peers in a p2p network exchange information about each other among other reasons to become resistant against the failure or replacement of the boot nodes of the network.
/// Furthermore, the [KamilataBehavior] does not reimplement the capabilities of libp2p's [Identify](libp2p::identify::Behaviour).
/// As a result, Kamilata only infers listen addresses of the peers we successfully dialed.
/// This means that the [Identify](libp2p::identify::Behaviour) protocol must be manually hooked up to Kademlia through calls to [KamilataBehavior::add_address].
/// If you choose not to use libp2p's [Identify](libp2p::identify::Behaviour), incoming connections will be accepted but we won't be able to relay queries to them.
/// This is the same approach as [Kademlia](libp2p::kad::Kademlia).
pub struct KamilataBehavior<const N: usize, D: Document<N>> {
    our_peer_id: PeerId,
    connected_peers: Vec<PeerId>,
    db: Arc<Db<N, D>>,

    rt_handle: tokio::runtime::Handle,

    /// Used to create new [BehaviourController]s
    control_msg_sender: Sender<BehaviourControlMessage>,
    /// Receiver of messages from [BehaviourController]s
    control_msg_receiver: Receiver<BehaviourControlMessage>,
    /// When a message is to be sent to a handler that is being dialed, it is temporarily stored here.
    pending_handler_events: BTreeMap<PeerId, HandlerInEvent>,
    /// When a message is ready to be dispatched to a handler, it is moved here.
    handler_event_queue: Vec<(PeerId, HandlerInEvent)>,

    task_counter: Counter,
    /// Tasks associated with task identifiers.  
    /// Reserved IDs:
    ///     none
    tasks: HashMap<usize, Task>,
}

impl<const N: usize, D: Document<N>> KamilataBehavior<N, D> {
    pub fn new(our_peer_id: PeerId) -> KamilataBehavior<N, D> {
        Self::new_with_config(our_peer_id, KamilataConfig::default())
    }

    pub fn new_with_config(our_peer_id: PeerId, config: KamilataConfig) -> KamilataBehavior<N, D> {
        let rt_handle = tokio::runtime::Handle::current();
        let (control_msg_sender, control_msg_receiver) = channel(100);

        KamilataBehavior {
            our_peer_id,
            connected_peers: Vec::new(),
            db: Arc::new(Db::new(config)),
            control_msg_sender,
            control_msg_receiver,
            pending_handler_events: BTreeMap::new(),
            handler_event_queue: Vec::new(),
            rt_handle,
            task_counter: Counter::new(0),
            tasks: HashMap::new(),
        }
    }

    pub async fn get_config(&self) -> KamilataConfig {
        self.db.get_config().await
    }

    pub async fn set_config(&self, config: KamilataConfig) {
        self.db.set_config(config).await
    }

    pub async fn insert_document(&self, document: D) {
        self.db.insert_document(document).await;
    }

    pub async fn insert_documents(&self, documents: Vec<D>) {
        self.db.insert_documents(documents).await;
    }

    pub async fn clear_documents(&self) {
        self.db.clear_documents().await;
    }

    pub async fn remove_document(&self, cid: &<D::SearchResult as SearchResult>::Cid) {
        self.db.remove_document(cid).await;
    }

    pub async fn remove_documents(&self, cids: &[&<D::SearchResult as SearchResult>::Cid]) {
        self.db.remove_documents(cids).await;
    }

    /// Starts a new search and returns an [handler](OngoingSearchControler) to control it.
    pub async fn search(&mut self, queries: impl Into<SearchQueries>) -> OngoingSearchController<D::SearchResult> {
        self.search_with_config(queries, SearchConfig::default()).await
    }

    /// Starts a new search with custom [SearchPriority] and returns an [handler](OngoingSearchControler) to control it.
    pub async fn search_with_priority(&mut self, queries: impl Into<SearchQueries>, priority: SearchPriority) -> OngoingSearchController<D::SearchResult> {
        self.search_with_config(queries, SearchConfig::default().with_priority(priority)).await
    }

    /// Starts a new search with custom [SearchConfig] and returns an [handler](OngoingSearchControler) to control it.
    pub async fn search_with_config(&mut self, queries: impl Into<SearchQueries>, config: SearchConfig) -> OngoingSearchController<D::SearchResult> {
        let queries = queries.into();
        let handler_messager = BehaviourController {
            sender: self.control_msg_sender.clone(),
        };
        let search_state = OngoingSearchState::new(queries, config);
        let (search_controler, search_follower) = search_state.into_pair::<D::SearchResult>();
        self.tasks.insert(self.task_counter.next() as usize, Box::pin(search(search_follower, handler_messager, Arc::clone(&self.db), self.our_peer_id)));
        search_controler
    }
}

impl<const N: usize, D: Document<N>> NetworkBehaviour for KamilataBehavior<N, D> {
    type ConnectionHandler = KamilataHandlerBuilder<N, D>;
    type OutEvent = KamilataEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        KamilataHandlerBuilder::new(self.our_peer_id, Arc::clone(&self.db))
    }

    fn on_swarm_event(&mut self, event: FromSwarm<Self::ConnectionHandler>) {
        match event {
            FromSwarm::ConnectionEstablished(info) => {
                self.connected_peers.push(info.peer_id);
                if let Some(msg) = self.pending_handler_events.remove(&info.peer_id) {
                    self.handler_event_queue.push((info.peer_id, msg));
                }
                if let ConnectedPoint::Dialer { address, .. } = info.endpoint {
                    let db2 = Arc::clone(&self.db);
                    let peer_id = info.peer_id;
                    let addr = address.to_owned();
                    tokio::spawn(async move {
                        db2.insert_address(peer_id, addr, true).await;
                    });
                }
            },
            FromSwarm::DialFailure(info) => {
                if let Some(peer_id) = info.peer_id {
                    self.pending_handler_events.remove(&peer_id);
                }
                warn!("{} Dial failure: {} with {:?}", self.our_peer_id, info.error, info.peer_id);
            },
            FromSwarm::ConnectionClosed(info) => {
                self.connected_peers.retain(|peer_id| peer_id != &info.peer_id);
                self.handler_event_queue.retain(|(peer_id, _)| peer_id != &info.peer_id);
                self.pending_handler_events.remove(&info.peer_id);
                let db2 = Arc::clone(&self.db);
                tokio::spawn(async move {
                    db2.remove_peer(&info.peer_id).await;
                });
            },
            _ => ()
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        // Message handlers first
        if let Some((peer_id, event)) = self.handler_event_queue.pop() {
            return Poll::Ready(
                NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler: libp2p::swarm::NotifyHandler::Any,
                    event
                }
            );
        }
        while let Poll::Ready(Some(control_message)) = self.control_msg_receiver.poll_recv(cx) {
            match control_message {
                BehaviourControlMessage::MessageHandler(peer_id, event) => return Poll::Ready(
                    NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        handler: libp2p::swarm::NotifyHandler::Any,
                        event
                    }
                ),
                BehaviourControlMessage::DialPeer(peer_id, addresses) => {
                    // Ignore if we are already connected to the peer.
                    if self.connected_peers.contains(&peer_id) {
                        continue;
                    }
                    return Poll::Ready(
                        NetworkBehaviourAction::Dial {
                            opts: libp2p::swarm::dial_opts::DialOpts::peer_id(peer_id).addresses(addresses).build(),
                            handler: KamilataHandlerBuilder::new(self.our_peer_id, Arc::clone(&self.db))
                        }
                    )
                },
                BehaviourControlMessage::DialPeerAndMessage(peer_id, addresses, event) => {
                    // Just notify the handler directly if we are already connected to the peer.
                    trace!("{} Dialing peer {peer_id} with addresses {addresses:?} and sending message", self.our_peer_id);
                    if self.connected_peers.contains(&peer_id) {
                        return Poll::Ready(
                            NetworkBehaviourAction::NotifyHandler {
                                peer_id,
                                handler: libp2p::swarm::NotifyHandler::Any,
                                event
                            }
                        );
                    }
                    self.pending_handler_events.insert(peer_id, event);
                    return Poll::Ready(
                        NetworkBehaviourAction::Dial {
                            opts: libp2p::swarm::dial_opts::DialOpts::peer_id(peer_id).addresses(addresses).build(),
                            handler: KamilataHandlerBuilder::new(self.our_peer_id, Arc::clone(&self.db))
                        }
                    );
                }
            }
        }

        // It seems this method gets called in a context where the tokio runtime does not exist.
        // We import that runtime so that we can rely on it.
        let _rt_enter_guard = self.rt_handle.enter();

        // Poll tasks
        for tid in self.tasks.keys().copied().collect::<Vec<_>>() {
            let task = self.tasks.get_mut(&tid).unwrap();

            match task.poll_unpin(cx) {
                Poll::Ready(output) => {
                    trace!("{} Task {tid} completed!", self.our_peer_id);
                    self.tasks.remove(&tid);

                    match output {
                        TaskOutput::None => (),
                    }
                }
                Poll::Pending => ()
            }
        }
        
        Poll::Pending
    }
}

/// Internal control messages send by [BehaviourController] to [KamilataBehavior]
#[derive(Debug)]
pub(crate) enum BehaviourControlMessage {
    MessageHandler(PeerId, HandlerInEvent),
    DialPeer(PeerId, Vec<Multiaddr>),
    DialPeerAndMessage(PeerId, Vec<Multiaddr>, HandlerInEvent),
}

/// A struct that allows to send messages to an [handler](ConnectionHandler)
#[derive(Clone)]
pub(crate) struct BehaviourController {
    sender: Sender<BehaviourControlMessage>,
}

impl BehaviourController {
    /// Sends a message to the handler.
    pub async fn message_handler(&self, peer_id: PeerId, message: HandlerInEvent) {
        self.sender.send(BehaviourControlMessage::MessageHandler(peer_id, message)).await.unwrap();
    }

    /// Requests behaviour to dial a peer.
    pub async fn dial_peer(&self, peer_id: PeerId, addresses: Vec<Multiaddr>) {
        self.sender.send(BehaviourControlMessage::DialPeer(peer_id, addresses)).await.unwrap();
    }

    /// Requests behaviour to dial a peer and send a message to it.
    pub async fn dial_peer_and_message(&self, peer_id: PeerId, addresses: Vec<Multiaddr>, message: HandlerInEvent) {
        self.sender.send(BehaviourControlMessage::DialPeerAndMessage(peer_id, addresses, message)).await.unwrap();
    }
}
