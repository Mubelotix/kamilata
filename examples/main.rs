use kamilata::prelude::*;

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

struct WordHasherImpl<const N: usize>;

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

#[derive(Debug)]
struct MovieResult {
    cid: String,
    desc: String,
}

impl SearchResult for MovieResult {
    type Cid = String;

    fn cid(&self) -> &Self::Cid {
        &self.cid
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&(self.cid.len() as u32).to_be_bytes());
        data.extend_from_slice(self.cid.as_bytes());
        data.extend_from_slice(&(self.desc.len() as u32).to_be_bytes());
        data.extend_from_slice(self.desc.as_bytes());
        data
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let mut bytes = bytes;
        let cid_len = u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize;
        bytes = &bytes[4..];
        let cid = String::from_utf8(bytes[..cid_len].to_vec()).unwrap();
        bytes = &bytes[cid_len..];
        let desc_len = u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize;
        bytes = &bytes[4..];
        let desc = String::from_utf8(bytes[..desc_len].to_vec()).unwrap();
        bytes = &bytes[desc_len..];
        assert!(bytes.is_empty());
        MovieResult {
            cid,
            desc,
        }
    }
}

#[derive(Debug)]
struct Movie<const N: usize> {
    cid: String,
    desc: String,
}

impl<const N: usize> Document<N> for Movie<N> {
    type SearchResult = MovieResult;
    type WordHasher = WordHasherImpl<N>;

    fn cid(&self) -> &<Self::SearchResult as SearchResult>::Cid {
        &self.cid
    }

    fn apply_to_filter(&self, filter: &mut Filter<N>) {
        self.desc.split(' ').filter(|w| w.len() >= 3).for_each(|word| {
            let hash = Self::WordHasher::hash_word(word);
            filter.set_bit(hash, true);
        });
    }

    fn search_result(&self, words: &[String], min_matching: usize) -> Option<Self::SearchResult> {
        let mut matching = 0;
        for word in words {
            if self.desc.contains(word) {
                matching += 1;
            }
        }
        if matching >= min_matching {
            Some(MovieResult {
                cid: self.cid.clone(),
                desc: self.desc.clone(),
            })
        } else {
            None
        }
    }
}

struct Client {
    local_key: Keypair,
    local_peer_id: PeerId,
    swarm: Swarm<KamilataBehavior<125000, Movie<125000>>>,
    addr: Multiaddr,
}

#[derive(Debug)]
enum ClientCommand {
    Search(String),
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

    async fn dial(&mut self, addr: Multiaddr) {
        println!("Dialing {addr:?}");
        self.swarm.dial(addr).unwrap();
    }

    async fn search(&mut self, query: &str) -> OngoingSearchControler<MovieResult> {
        let words = query.split(' ').filter(|w| w.len() >= 3).map(|w| w.to_string()).collect();
        self.swarm.behaviour_mut().search(words).await
    }

    async fn run(mut self, mut command: Receiver<ClientCommand>) {
        loop {
            let recv = Box::pin(command.recv());
            let value = futures::future::select(recv, self.swarm.select_next_some()).await;
            match value {
                future::Either::Left((Some(command), _)) => match command {
                    ClientCommand::Search(query) => {
                        let mut controler = self.search(&query).await;
                        tokio::spawn(async move {
                            let mut results = Vec::new();
                            while let Some(result) = controler.recv().await {
                                results.push(result);
                            }
                            println!("Found {} search results: {results:#?}", results.len());
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
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client1 = Client::init(1000).await;
    client1.swarm.behaviour().insert_documents(vec![
        Movie {
            cid: "V for Vendetta".to_string(),
            desc: "In a future British dystopian society, a shadowy freedom fighter, known only by the alias of \"V\", plots to overthrow the tyrannical government - with the help of a young woman.".to_string(),
        },
        Movie {
            cid: "The Matrix".to_string(),
            desc: "When a beautiful stranger leads computer hacker Neo to a forbidding underworld, he discovers the shocking truth--the life he knows is the elaborate deception of an evil cyber-intelligence.".to_string(),
        },
        Movie {
            cid: "Revolution of Our Times".to_string(),
            desc: "Due to political restrictions in Hong Kong, this documentary following protestors since 2019, is broken into pieces, each containing interviews and historical context of the conflict.".to_string(),
        },
        Movie {
            cid: "The Social Dilemma".to_string(),
            desc: "Explores the dangerous human impact of social networking, with tech experts sounding the alarm on their own creations.".to_string(),
        },
        Movie {
            cid: "The Hunger Games".to_string(),
            desc: "Katniss Everdeen voluntarily takes her younger sister's place in the Hunger Games: a televised competition in which two teenagers from each of the twelve Districts of Panem are chosen at random to fight to the death.".to_string(),
        }
    ]).await;

    let addr = client1.addr.clone();
    let (sender1, receiver1) = channel(6);
    let h1 = tokio::spawn(client1.run(receiver1));

    let mut client2 = Client::init(1001).await;
    client2.dial(addr).await;
    let (sender2, receiver2) = channel(6);
    let h2 = tokio::spawn(client2.run(receiver2));

    sleep(Duration::from_secs(5)).await;

    sender2.send(ClientCommand::Search("Hunger".to_string())).await.unwrap();

    join(h1, h2).await.0.unwrap();

    Ok(())
}
