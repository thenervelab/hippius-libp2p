use async_std::task;
use libp2p::{
    core::transport::upgrade, identity, multiaddr::Protocol, noise, relay, swarm::SwarmBuilder,
    tcp::TcpConfig, webrtc::WebRtcTransport, websocket::WsConfig, yamux::YamuxConfig, Multiaddr,
    PeerId, Swarm, Transport,
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
    let tcp_transport = TcpConfig::new();
    let websocket_transport = WsConfig::new(tcp_transport.clone());
    let webrtc_transport = WebRtcTransport::new(local_key.clone()).unwrap();
    let transport = tcp_transport
        .or_transport(websocket_transport)
        .or_transport(webrtc_transport)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(local_key.clone()).unwrap())
        .multiplex(YamuxConfig::default())
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
    let listen_tcp: Multiaddr = Protocol::Ip4([0, 0, 0, 0].into())
        .and_then(Protocol::Tcp(4501))
        .into();
    let listen_ws: Multiaddr = Protocol::Ip4([0, 0, 0, 0].into())
        .and_then(Protocol::Tcp(4502))
        .and_then(Protocol::Ws)
        .into();
    let listen_webrtc: Multiaddr = Protocol::Ip4([0, 0, 0, 0].into())
        .and_then(Protocol::Udp(4503))
        .and_then(Protocol::WebRtc)
        .into();

    Swarm::listen_on(&mut swarm, listen_tcp).unwrap();
    Swarm::listen_on(&mut swarm, listen_ws).unwrap();
    Swarm::listen_on(&mut swarm, listen_webrtc).unwrap();

    println!("Relay node listening on TCP: {}", listen_tcp);
    println!("Relay node listening on WebSocket: {}", listen_ws);
    println!("Relay node listening on WebRTC: {}", listen_webrtc);

    // Handle incoming connections
    loop {
        match swarm.next().await {
            Some(event) => println!("Relay event: {:?}", event),
            None => break,
        }
    }
}
