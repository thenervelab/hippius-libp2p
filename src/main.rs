use std::{collections::hash_map::DefaultHasher, error::Error, hash::{Hash, Hasher}, time::Duration};
use futures::stream::StreamExt;
use libp2p::{
    core::Multiaddr,
    gossipsub::{self, IdentTopic, MessageId},
    mdns, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
    webrtc,
    PeerId,
};
use tokio::select;

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    webrtc: webrtc::tokio::Transport, // WebRTC for relaying client connections
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Generate a new key pair for the node
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {local_peer_id}");

    // WebRTC transport for relaying clients
    let webrtc_transport = webrtc::tokio::Transport::new(webrtc::Config::new(&local_key));

    // Set up Gossipsub with content-addressable message IDs
    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        MessageId::from(s.finish().to_string())
    };

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .build()
        .expect("Valid gossipsub config");

    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(local_key.clone()),
        gossipsub_config,
    )
    .expect("Valid gossipsub behaviour");

    // Enable mDNS for peer discovery
    let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;

    // Combine Gossipsub, mDNS, and WebRTC into one behaviour
    let behaviour = MyBehaviour {
        gossipsub,
        mdns,
        webrtc: webrtc_transport,
    };

    // Build the swarm
    let mut swarm = libp2p::Swarm::builder(behaviour, local_key.clone())
        .executor(Box::new(|fut| tokio::spawn(fut)))
        .build();

    // Listen on TCP, QUIC, and WebRTC
    swarm.listen_on("/ip4/0.0.0.0/tcp/4001".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/udp/4002/quic-v1".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/udp/4003/webrtc".parse()?)?;

    // Gossipsub topic
    let topic = IdentTopic::new("yjs-sync");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    println!("Server is running. Listening on:");
    for address in libp2p::Swarm::listeners(&swarm) {
        println!("{address}");
    }

    // Main event loop
    loop {
        select! {
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(gossipsub::Event::Message {
                    propagation_source,
                    message_id,
                    message,
                }) => {
                    println!(
                        "Received message: {:?} from {:?}",
                        String::from_utf8_lossy(&message.data),
                        propagation_source
                    );
                }
                SwarmEvent::Behaviour(mdns::Event::Discovered(peers)) => {
                    for (peer_id, _) in peers {
                        println!("Discovered peer: {peer_id}");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on: {address}");
                }
                _ => {}
            }
        }
    }
}
