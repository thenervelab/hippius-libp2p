
use libp2p::{
    core::upgrade,
    identity,
    noise::{NoiseConfig, Keypair},
    ping,
    swarm::{Swarm, SwarmEvent, Config},
    tcp,
    websocket,
    yamux::YamuxConfig,
    Multiaddr, PeerId, Transport,
    SwarmBuilder,
    webrtc,
    Multiaddr, PeerId, Transport,
};
use std::error::Error;
use tracing::info;
use tracing_subscriber::EnvFilter;
use libp2p::swarm::NetworkBehaviour;
use libp2p_swarm_derive::NetworkBehaviour;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    info!("Local peer id: {:?}", local_peer_id);

    let transport = build_transport(local_key).await?;

    let mut swarm = {
        let ping = ping::Behaviour::new(ping::Config::default());
        let behaviour = Behaviour {
            ping,
        };
        SwarmBuilder::with_options(transport, behaviour, local_peer_id, Config::with_executor(tokio::task::spawn)).build()
    };

    let addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    swarm.listen_on(addr)?;

    loop {
        match swarm.next_event().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {:?}", address);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Ping(ping::Event {
                peer,
                result: Result::Ok(ping::Success::Ping { rtt, .. }),
                ..
            })) => {
                info!("Ping: rtt to {:?} is {:?}", peer, rtt);
            }
            _ => {}
        }
    }
}

async fn build_transport(
    local_key: identity::Keypair,
) -> Result<impl Transport<Output = (PeerId, upgrade::Negotiated<impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin>)>, Box<dyn Error>> {
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));
    let ws_transport = websocket::tokio::Transport::new(websocket::WsConfig::default());
    let webrtc_transport = webrtc::tokio::Transport::new(webrtc::Config::default(), local_key.clone());

    let transport = tcp_transport
        .or_transport(ws_transport)
        .or_transport(webrtc_transport);

    let noise_config = NoiseConfig::xx(Keypair::generate());
    let mux_config = YamuxConfig::default();

    Ok(transport
        .upgrade(upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(mux_config)
        .boxed())
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourEvent")]
struct Behaviour {
    ping: ping::Behaviour,
}

#[derive(Debug)]
enum BehaviourEvent {
    Ping(ping::Event),
}
