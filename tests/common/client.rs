
use futures::future;
use libp2p::{identity::{self, Keypair}, core::transport::MemoryTransport, PeerId, Transport, Swarm, Multiaddr, swarm::SwarmEvent};

use tokio::{
    sync::{
        mpsc::*,
        oneshot::{
            channel as oneshot_channel, Receiver as OneshotReceiver, Sender as OneshotSender,
        },
        RwLock,
    },
};
use futures::StreamExt;
use log::*;
use super::*;

pub async fn memory_transport(
    keypair: identity::Keypair,
) -> std::io::Result<libp2p::core::transport::Boxed<(PeerId, libp2p::core::muxing::StreamMuxerBox)>> {
    let transport = MemoryTransport::default();

    let noise_keys = libp2p::noise::Keypair::<libp2p::noise::X25519Spec>::new()
        .into_authentic(&keypair)
        .expect("Signing libp2p-noise static DH keypair failed.");

    Ok(transport
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(libp2p::noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(libp2p::core::upgrade::SelectUpgrade::new(
            libp2p::yamux::YamuxConfig::default(),
            libp2p::mplex::MplexConfig::default(),
        ))
        .timeout(std::time::Duration::from_secs(20))
        .boxed())
}

pub struct Client {
    local_key: Keypair,
    local_peer_id: PeerId,
    swarm: Swarm<KamilataBehavior<125000, Movie>>,
    addr: Multiaddr,
}

#[derive(Debug)]
pub enum ClientCommand {
    Search {
        query: String,
        sender: OneshotSender<SearchResults<Movie>>,
    },
}

pub struct ClientController {
    sender: Sender<ClientCommand>,
}

impl ClientController {
    pub async fn search(&self, query: impl Into<String>) -> SearchResults<Movie> {
        let (sender, receiver) = oneshot_channel();
        self.sender.send(ClientCommand::Search {
            query: query.into(),
            sender,
        }).await.unwrap();
        receiver.await.unwrap()
    }
}

impl Client {
    pub async fn init(n: usize) -> Self {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
    
        let transport = memory_transport(local_key.clone()).await.expect("Failed to build transport");

        // Create a ping network behaviour.
        //
        // For illustrative purposes, the ping protocol is configured to
        // keep the connection alive, so a continuous sequence of pings
        // can be observed.
        let behaviour = KamilataBehavior::new(local_peer_id);
    
        let mut swarm = Swarm::new(transport, behaviour, local_peer_id);
    
        // Tell the swarm to listen on all interfaces and a random, OS-assigned
        // port.
        let addr: Multiaddr = format!("/memory/{n}").parse().unwrap();
        swarm.listen_on(addr.clone()).unwrap();
    
        Client {
            local_key,
            local_peer_id,
            swarm,
            addr,
        }
    }

    pub fn addr(&self) -> &Multiaddr {
        &self.addr
    }

    pub fn peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    pub fn behavior(&self) -> &KamilataBehavior<125000, Movie> {
        self.swarm.behaviour()
    }

    pub fn behavior_mut(&mut self) -> &mut KamilataBehavior<125000, Movie> {
        self.swarm.behaviour_mut()
    }

    pub fn swarm(&self) -> &Swarm<KamilataBehavior<125000, Movie>> {
        &self.swarm
    }

    pub fn swarm_mut(&mut self) -> &mut Swarm<KamilataBehavior<125000, Movie>> {
        &mut self.swarm
    }

    pub fn run(mut self) -> ClientController {
        let (sender, mut receiver) = channel(1);
        tokio::spawn(async move {
            loop {
                let recv = Box::pin(receiver.recv());
                let value = futures::future::select(recv, self.swarm.select_next_some()).await;
                match value {
                    future::Either::Left((Some(command), _)) => match command {
                        ClientCommand::Search { query, sender } => {
                            let words = query.split(' ').filter(|w| w.len() >= 3).map(|w| w.to_string()).collect();
                            let mut controler = self.swarm.behaviour_mut().search(words).await;
                    
                            tokio::spawn(async move {
                                let mut hits = Vec::new();
                                while let Some(hit) = controler.recv().await {
                                    hits.push(hit);
                                }
                                let mut results = controler.finish().await;
                                results.hits = hits;
                                sender.send(results).unwrap();
                            });
                        },
                    },
                    future::Either::Left((None, _)) => break,
                    future::Either::Right((event, _)) => match event {
                        SwarmEvent::Behaviour(e) => info!("{} produced behavior event {e:?}", self.local_peer_id),
                        SwarmEvent::NewListenAddr { listener_id, address } => debug!("{} is listening on {address:?} (listener id: {listener_id:?})", self.local_peer_id),
                        _ => ()
                    },
                }
            }
        });
        ClientController {
            sender,
        }
    }
}
