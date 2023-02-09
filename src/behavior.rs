use crate::prelude::*;

#[derive(Debug)]
pub enum KamilataEvent {

}

#[derive(Debug)]
pub enum BehaviourControlMessage {
    MessageHandler(PeerId, HandlerInEvent),
    DialPeer(PeerId, Vec<Multiaddr>),
    DialPeerAndMessage(PeerId, Vec<Multiaddr>, HandlerInEvent),
}

/// A struct that allows to send messages to an [handler](ConnectionHandler)
#[derive(Clone)]
pub struct BehaviourController {
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

pub struct KamilataBehavior<const N: usize, D: Document<N>> {
    our_peer_id: PeerId,
    connected_peers: Vec<PeerId>,
    db: Arc<Db<N, D>>,
    control_msg_sender: Sender<BehaviourControlMessage>,
    control_msg_receiver: Receiver<BehaviourControlMessage>,
    pending_handler_events: BTreeMap<PeerId, HandlerInEvent>,
    handler_event_queue: Vec<(PeerId, HandlerInEvent)>,
    
    rt_handle: tokio::runtime::Handle,

    task_counter: Counter,
    /// Tasks associated with task identifiers.  
    /// Reserved IDs:
    ///     none
    tasks: HashMap<usize, Task>,
}

impl<const N: usize, D: Document<N>> KamilataBehavior<N, D> {
    pub fn new(our_peer_id: PeerId) -> KamilataBehavior<N, D> {
        let rt_handle = tokio::runtime::Handle::current();
        let (control_msg_sender, control_msg_receiver) = channel(100);

        KamilataBehavior {
            our_peer_id,
            connected_peers: Vec::new(),
            db: Arc::new(Db::new()),
            control_msg_sender,
            control_msg_receiver,
            pending_handler_events: BTreeMap::new(),
            handler_event_queue: Vec::new(),
            rt_handle,
            task_counter: Counter::new(0),
            tasks: HashMap::new(),
        }
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
    pub async fn search(&mut self, words: Vec<String>) -> OngoingSearchControler<D::SearchResult> {
        let handler_messager = BehaviourController {
            sender: self.control_msg_sender.clone(),
        };
        let words_len = words.len();
        let search_state = OngoingSearchState::new(vec![(words, words_len)]);
        let (search_controler, search_follower) = search_state.into_pair::<D::SearchResult>();
        self.tasks.insert(self.task_counter.next() as usize, Box::pin(search(search_follower, handler_messager, Arc::clone(&self.db), self.our_peer_id)));
        search_controler
    }
}

impl<const N: usize, D: Document<N>> NetworkBehaviour for KamilataBehavior<N, D> {
    type ConnectionHandler = KamilataHandlerProto<N, D>;
    type OutEvent = KamilataEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        KamilataHandlerProto::new(self.our_peer_id, Arc::clone(&self.db))
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: HandlerOutEvent,
    ) {
        todo!()
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
                            handler: KamilataHandlerProto::new(self.our_peer_id, Arc::clone(&self.db))
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
                            handler: KamilataHandlerProto::new(self.our_peer_id, Arc::clone(&self.db))
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
