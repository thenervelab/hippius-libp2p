use libp2p::{
    core::{
        muxing::StreamMuxerBox,
        transport::{Boxed, Transport},
        upgrade,
    },
    identity,
    noise,
    swarm::{Swarm, SwarmEvent},
    tcp,
    websocket,
    webrtc,
    Multiaddr, PeerId, SwarmBuilder,
};
use std::error::Error;
use futures::stream::StreamExt;
use libp2p_mplex::MplexConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    let transport = build_transport(local_key.clone())?;

    let behaviour = libp2p::swarm::dummy::Behaviour::new();

    let mut swarm =
        SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_other_transport(move |_| transport)
            .with_behaviour(|_| behaviour)
            .build();


    let ws_listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0/ws".parse()?;
    swarm.listen_on(ws_listen_addr)?;

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {:?}", address);
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


fn build_transport(
    local_key: identity::Keypair,
) -> Result<Boxed<()>, Box<dyn Error>> {
    let tcp_transport = tcp::tokio::Transport::new(
        tcp::Config::default().nodelay(true),
    );
    let ws_transport = websocket::WsConfig::new(
        tcp::tokio::Transport::new(
            tcp::Config::default().nodelay(true),
        )
    );
    let webrtc_transport = webrtc::tokio::Transport::new(
        webrtc::Config::new(&local_key)
    );

    let noise_config = noise::Config::new(&local_key);

    let transport = tcp_transport
        .or_transport(ws_transport)
        .or_transport(webrtc_transport)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(MplexConfig::new())
        .boxed();

    Ok(transport)
}
