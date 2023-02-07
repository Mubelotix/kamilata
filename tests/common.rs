use std::ops::Deref;

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

#[derive(Debug)]
pub struct MovieResult {
    pub cid: String,
    pub desc: String,
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
pub struct Movie<const N: usize> {
    pub cid: String,
    pub desc: String,
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

pub struct Client {
    local_key: Keypair,
    local_peer_id: PeerId,
    swarm: Swarm<KamilataBehavior<125000, Movie<125000>>>,
    addr: Multiaddr,
}

#[derive(Debug)]
pub enum ClientCommand {
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

    pub fn addr(&self) -> &Multiaddr {
        &self.addr
    }

    pub fn behavior(&self) -> &KamilataBehavior<125000, Movie<125000>> {
        self.swarm.behaviour()
    }

    pub fn behavior_mut(&mut self) -> &mut KamilataBehavior<125000, Movie<125000>> {
        self.swarm.behaviour_mut()
    }

    pub fn swarm(&self) -> &Swarm<KamilataBehavior<125000, Movie<125000>>> {
        &self.swarm
    }

    pub fn swarm_mut(&mut self) -> &mut Swarm<KamilataBehavior<125000, Movie<125000>>> {
        &mut self.swarm
    }

    async fn search(&mut self, query: &str) -> OngoingSearchControler<MovieResult> {
        let words = query.split(' ').filter(|w| w.len() >= 3).map(|w| w.to_string()).collect();
        self.swarm.behaviour_mut().search(words).await
    }

    pub async fn run(mut self, mut command: Receiver<ClientCommand>) {
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

impl Deref for Client {
    type Target = KamilataBehavior<125000, Movie<125000>>;

    fn deref(&self) -> &Self::Target {
        self.swarm.behaviour()
    }
}
