//! Ping example
//!
//! See ../src/tutorial.rs for a step-by-step guide building the example below.
//!
//! In the first terminal window, run:
//!
//! ```sh
//! cargo run --example ping
//! ```
//!
//! It will print the PeerId and the listening addresses, e.g. `Listening on
//! "/ip4/0.0.0.0/tcp/24915"`
//!
//! In the second terminal window, start a new instance of the example with:
//!
//! ```sh
//! cargo run --example ping -- /ip4/127.0.0.1/tcp/24915
//! ```
//!
//! The two nodes establish a connection, negotiate the ping protocol
//! and begin pinging each other.

mod kamilata;
use kamilata::*;
mod prelude;
mod packets;
mod handler;
mod counter;
mod filters;
mod config;
use prelude::*;

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
    swarm: Swarm<Kamilata>,
    addr: Multiaddr,
}

impl Client {
    pub async fn init(n: usize) -> Self {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        println!("Local peer id: {:?}", local_peer_id);
    
        let transport = memory_transport(local_key.clone()).await.expect("Failed to build transport");

        // Create a ping network behaviour.
        //
        // For illustrative purposes, the ping protocol is configured to
        // keep the connection alive, so a continuous sequence of pings
        // can be observed.
        let behaviour = Kamilata::new();
    
    
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

    fn dial(&mut self, addr: Multiaddr) {
        println!("Dialing {:?}", addr);
        self.swarm.dial(addr).unwrap();
    }

    async fn run(mut self) {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {:?}", address),
                SwarmEvent::Behaviour(event) => println!("{:?}", event),
                _ => {}
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client1 = Client::init(1000).await;
    let addr = client1.addr.clone();
    let h1 = tokio::spawn(client1.run());

    let mut client2 = Client::init(1001).await;
    client2.dial(addr);
    let h2 = tokio::spawn(client2.run());

    join(h1, h2).await.0.unwrap();

    Ok(())
}
