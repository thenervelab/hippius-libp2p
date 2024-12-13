use async_std::task;
use libp2p::{
    core::transport::upgrade,
    identity,
    multiaddr::Protocol,
    noise,
    relay,
    swarm::{SwarmEvent},
    tcp::{self, Config as TcpConfig},
    webrtc::{self, Transport as WebRtcTransport},
    websocket,
    yamux,
    Multiaddr,
    PeerId,
    Swarm,
    Transport,
    SwarmBuilder,
};
use std::{fs, path::Path};

const KEYPAIR_FILE: &str = "relay_keypair";

fn load_or_generate_keypair() -> identity::Keypair {
    if Path::new(KEYPAIR_FILE).exists() {
        let keypair_bytes = fs::read(KEYPAIR_FILE).expect("Failed to read keypair file");
        identity::Keypair::from_protobuf_encoding(&keypair_bytes)
            .expect("Failed to decode keypair")
    } else {
        let keypair = identity::Keypair::generate_ed25519();
        let keypair_bytes = keypair
            .to_protobuf_encoding()
            .expect("Failed to encode keypair");
        fs::write(KEYPAIR_FILE, &keypair_bytes).expect("Failed to save keypair");
        keypair
    }
}

#[async_std::main]
async fn main() {
    // Load or generate the keypair
    let local_key = load_or_generate_keypair();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Relay Node ID: {}", local_peer_id);

    // Configure transport: Combine TCP, WebSocket, and WebRTC
    let tcp_transport = tcp::Config::new();
    let tcp_transport = tcp::TcpTransport::new(tcp_transport);
    let websocket_transport = websocket::WsConfig::new(tcp_transport.clone());
    let webrtc_transport = WebRtcTransport::new(local_key.clone());
    let transport = tcp_transport
        .or_transport(websocket_transport)
        .or_transport(webrtc_transport)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::Config::new(&local_key).unwrap().into_authenticated())
        .multiplex(yamux::Config::default())
        .boxed();

    // Set up the relay behavior
    let relay_behavior = relay::Relay::new(local_peer_id.clone());

    // Build the Swarm
    let mut swarm = SwarmBuilder::new(transport, relay_behavior, local_peer_id.clone())
        .executor(Box::new(|fut| {
            task::spawn(fut);
        }))
        .build();

    // Listen on new ports to avoid IPFS conflicts
    let listen_tcp: Multiaddr = "/ip4/0.0.0.0/tcp/4501".parse().unwrap();
    let listen_ws: Multiaddr = "/ip4/0.0.0.0/tcp/4502/ws".parse().unwrap();
    let listen_webrtc: Multiaddr = "/ip4/0.0.0.0/udp/4503/webrtc".parse().unwrap();

    Swarm::listen_on(&mut swarm, listen_tcp).unwrap();
    Swarm::listen_on(&mut swarm, listen_ws).unwrap();
    Swarm::listen_on(&mut swarm, listen_webrtc).unwrap();

    println!("Relay node listening on TCP: {}", listen_tcp);
    println!("Relay node listening on WebSocket: {}", listen_ws);
    println!("Relay node listening on WebRTC: {}", listen_webrtc);

    // Handle incoming connections
    loop {
        if let Some(event) = swarm.next().await {
            match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {:?}", address);
                }
                event => println!("Relay event: {:?}", event),
            }
        }
    }
}
