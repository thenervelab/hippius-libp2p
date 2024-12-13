use async_std::task;
use libp2p::{
    core::{
        muxing::StreamMuxerBox,
        transport::{Boxed, Transport},
        upgrade,
    },
    identity,
    noise,
    relay,
    swarm::{Swarm, SwarmEvent},
    tcp,
    websocket,
    Multiaddr, PeerId, SwarmBuilder,
};
use libp2p::webrtc;
use std::error::Error;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    let transport = build_transport(local_key).await?;

    let mut swarm = SwarmBuilder::with_tokio_executor(
        transport,
        Default::default(),
        local_peer_id,
    )
    .build();

    let addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    swarm.listen_on(addr)?;

    loop {
        match swarm.next_event().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {:?}", address);
            }
            SwarmEvent::IncomingConnection { .. } => {
                println!("Incoming connection");
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                println!("Connection established with {:?}", peer_id);
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                println!("Connection closed with {:?}", peer_id);
            }
            _ => {}
        }
    }
}

async fn build_transport(
    local_key: identity::Keypair,
) -> Result<Boxed<(PeerId, StreamMuxerBox)>, Box<dyn Error>> {
    let tcp_transport = tcp::TcpTransport::new(tcp::Config::new());
    let ws_transport = websocket::WsConfig::new(tcp_transport.clone());
    let webrtc_transport = webrtc::Transport::new(
        local_key,
        webrtc::Config::new(),
    );

    let noise_config = noise::Config::new(&local_key);

    let transport = tcp_transport
        .or_transport(ws_transport)
        .or_transport(webrtc_transport)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(libp2p::mplex::MplexConfig::new())
        .boxed();

    Ok(transport)
}
