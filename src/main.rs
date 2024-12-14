
use libp2p::{
    core::upgrade,
    identity,
    noise,
    ping,
    swarm::{Swarm, SwarmEvent},
    tcp,
    webrtc,
    websocket,
    yamux,
    Multiaddr, PeerId, Transport,
};
use libp2p_webrtc_direct::WebRtcDirect;
use std::error::Error;
use tracing::info;
use tracing_subscriber::EnvFilter;

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
        let ping = ping::Behaviour::new(ping::Config::new());
        let webrtc_direct = WebRtcDirect::new();
        let behaviour = Behaviour {
            ping,
            webrtc_direct,
        };
        Swarm::new(transport, behaviour, local_peer_id)
    };

    let addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    swarm.listen_on(addr)?;

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {:?}", address);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Ping(ping::Event {
                peer_id,
                result: Result::Ok(ping::Success::Ping { rtt }),
            })) => {
                info!("Ping: rtt to {:?} is {:?}", peer_id, rtt);
            }
            SwarmEvent::Behaviour(BehaviourEvent::WebRtcDirect(
                libp2p_webrtc_direct::Event::Connected { peer_id },
            )) => {
                info!("WebRTC Direct connected to {:?}", peer_id);
            }
            SwarmEvent::Behaviour(BehaviourEvent::WebRtcDirect(
                libp2p_webrtc_direct::Event::Incoming { peer_id, .. },
            )) => {
                info!("WebRTC Direct incoming connection from {:?}", peer_id);
            }
            SwarmEvent::Behaviour(BehaviourEvent::WebRtcDirect(
                libp2p_webrtc_direct::Event::Disconnected { peer_id },
            )) => {
                info!("WebRTC Direct disconnected from {:?}", peer_id);
            }
            _ => {}
        }
    }
}

async fn build_transport(
    local_key: identity::Keypair,
) -> Result<impl Transport<Output = (PeerId, upgrade::Negotiated<impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin>)>, Box<dyn Error>> {
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));
    let ws_transport = websocket::tokio::Transport::new(websocket::Config::default());
    let webrtc_transport = webrtc::tokio::Transport::new(webrtc::Config::default(), local_key.clone());

    let transport = tcp_transport
        .or_transport(ws_transport)
        .or_transport(webrtc_transport);

    let noise_config = noise::NoiseConfig::xx(noise::Keypair::new().unwrap());
    let mux_config = yamux::YamuxConfig::default();

    Ok(transport
        .upgrade(upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(mux_config)
        .boxed())
}

#[derive(libp2p::swarm::NetworkBehaviour)]
struct Behaviour {
    ping: ping::Behaviour,
    webrtc_direct: WebRtcDirect,
}

#[derive(Debug)]
enum BehaviourEvent {
    Ping(ping::Event),
    WebRtcDirect(libp2p_webrtc_direct::Event),
}

impl From<ping::Event> for BehaviourEvent {
    fn from(event: ping::Event) -> Self {
        BehaviourEvent::Ping(event)
    }
}

impl From<libp2p_webrtc_direct::Event> for BehaviourEvent {
    fn from(event: libp2p_webrtc_direct::Event) -> Self {
        BehaviourEvent::WebRtcDirect(event)
    }
}
