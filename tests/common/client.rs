
use futures::future;
use libp2p::{identity::{self, Keypair}, core::transport::MemoryTransport, PeerId, Transport, Swarm, Multiaddr, swarm::SwarmEvent};

use tokio::{
    sync::{
        mpsc::*,
        oneshot::{
            channel as oneshot_channel, Sender as OneshotSender,
        }
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
        .multiplex(libp2p::mplex::MplexConfig::default())
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
        queries: SearchQueries,
        sender: OneshotSender<SearchResults<Movie>>,
        config: SearchConfig,
    },
}

pub struct ClientController {
    sender: Sender<ClientCommand>,
}

impl ClientController {
    pub async fn search(&self, queries: impl Into<SearchQueries>) -> SearchResults<Movie> {
        self.search_with_config(queries, SearchConfig::default()).await
    }

    pub async fn search_with_priority(&self, queries: impl Into<SearchQueries>, priority: SearchPriority) -> SearchResults<Movie> {
        self.search_with_config(queries, SearchConfig::default().with_priority(priority)).await
    }

    pub async fn search_with_config(&self, queries: impl Into<SearchQueries>, config: SearchConfig) -> SearchResults<Movie> {
        let (sender, receiver) = oneshot_channel();
        self.sender.send(ClientCommand::Search {
            queries: queries.into(),
            sender,
            config,
        }).await.unwrap();
        receiver.await.unwrap()
    }
}

impl Client {
    pub async fn init() -> Self {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
    
        let transport = libp2p::tokio_development_transport(local_key.clone()).unwrap();

        // Create a ping network behaviour.
        //
        // For illustrative purposes, the ping protocol is configured to
        // keep the connection alive, so a continuous sequence of pings
        // can be observed.
        let behaviour = KamilataBehavior::new(local_peer_id);
    
        let mut swarm = Swarm::with_tokio_executor(transport, behaviour, local_peer_id);
    
        // Tell the swarm to listen on all interfaces and a random, OS-assigned port.
        let mut addr: Option<Multiaddr> = None;
        for _ in 0..20 {
            let n: usize = rand::random();
            let addr2: Multiaddr = format!("/ip4/127.0.0.1/tcp/{}", 11201+n%4000).parse().unwrap();
            match swarm.listen_on(addr2.clone()) {
                Ok(_) => {
                    addr = Some(addr2);
                    break;
                }
                Err(err) => eprintln!("Failed to listen on {addr2} {err}"),
            }
        }
    
        Client {
            local_key,
            local_peer_id,
            swarm,
            addr: addr.expect("Failed to listen on any addr"),
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
                        ClientCommand::Search { queries, sender, config } => {
                            let mut controler = self.swarm.behaviour_mut().search_with_config(queries, config).await;
                    
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
