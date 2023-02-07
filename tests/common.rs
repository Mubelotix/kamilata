use tokio::sync::oneshot::{channel as oneshot_channel, Sender as OneshotSender};
use kamilata::prelude::*;
use serde::{Serialize, Deserialize};

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

pub struct WordHasherImpl<const N: usize>;

impl<const N: usize> WordHasher<N> for WordHasherImpl<N> {
    fn hash_word(word: &str) -> usize {
        let mut result = 1usize;
        const RANDOM_SEED: [usize; 16] = [542587211452, 5242354514, 245421154, 4534542154, 542866467, 545245414, 7867569786914, 88797854597, 24542187316, 645785447, 434963879, 4234274, 55418648642, 69454242114688, 74539841, 454214578213];
        for c in word.bytes() {
            for i in 0..8 {
                result = result.overflowing_mul(c as usize + RANDOM_SEED[i*2]).0;
                result = result.overflowing_add(c as usize + RANDOM_SEED[i*2+1]).0;
            }
        }
        result % (N * 8)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Movie {
    id: usize,
    title: String,
    overview: String,
    genres: Vec<String>,
    poster: String,
    release_date: i64,
}

impl Movie {
    fn full_text(&self) -> String {
        let mut full_text = String::new();
        full_text.push_str(&self.title);
        full_text.push(' ');
        full_text.push_str(&self.overview);
        full_text.push(' ');
        for genre in &self.genres {
            full_text.push_str(genre);
            full_text.push(' ');
        }
        full_text = full_text.to_lowercase();
        full_text
    }

    pub fn words(&self) -> Vec<String> {
        self.full_text().split(|c: char| c.is_whitespace() || c.is_ascii_punctuation()).filter(|w| w.len() >= 3).map(|w| w.to_string()).collect()
    }
}

impl SearchResult for Movie {
    type Cid = usize;

    fn cid(&self) -> &Self::Cid {
        &self.id
    }

    fn into_bytes(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        serde_json::from_slice(bytes).unwrap()
    }
}

impl<const N: usize> Document<N> for Movie {
    type SearchResult = Movie;
    type WordHasher = WordHasherImpl<N>;

    fn cid(&self) -> &<Self::SearchResult as SearchResult>::Cid {
        &self.id
    }

    fn apply_to_filter(&self, filter: &mut Filter<N>) {
        self.words().iter().for_each(|word| {
            let idx = Self::WordHasher::hash_word(word);
            filter.set_bit(idx, true)
        });
    }

    fn search_result(&self, query_words: &[String], min_matching: usize) -> Option<Self::SearchResult> {
        let words = self.words();

        let mut matches = 0;
        for word in query_words {
            if words.contains(word) {
                matches += 1;
            }
        }

        if min_matching <= matches {
            Some(self.to_owned())
        } else {
            None
        }
    }
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
        sender: OneshotSender<Vec<Movie>>,
    },
}

pub struct ClientController {
    sender: Sender<ClientCommand>,
}

impl ClientController {
    pub async fn search(&self, query: impl Into<String>) -> Vec<Movie> {
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
        println!("Local peer id: {local_peer_id:?}");
    
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
                                let mut results = Vec::new();
                                while let Some(result) = controler.recv().await {
                                    results.push(result.0);
                                }
                                println!("Found {} search results: {results:#?}", results.len());
                                sender.send(results).unwrap();
                            });
                        },
                    },
                    future::Either::Left((None, _)) => break,
                    future::Either::Right((event, _)) => match event {
                        SwarmEvent::Behaviour(e) => println!("{} produced behavior event {e:?}", self.local_peer_id),
                        SwarmEvent::NewListenAddr { listener_id, address } => println!("{} is listening on {address:?} (listener id: {listener_id:?})", self.local_peer_id),
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
