use libp2p::{
    core::{
        muxing::StreamMuxerBox,
        transport::{Boxed, Transport},
        upgrade,
    },
    identity,
    noise,
    SwarmBuilder,
    swarm::{SwarmEvent, NetworkBehaviour},
    tcp,
    websocket,
    Multiaddr, PeerId,
    futures::StreamExt,
};
use libp2p_mplex::MplexConfig;
use std::error::Error;
use std::time::Duration;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "MyBehaviourEvent", event_process = false)]
struct MyBehaviour {
    #[behaviour(ignore)]
    _priv: (),
}

#[derive(Debug)]
enum MyBehaviourEvent {}

impl Default for MyBehaviour {
    fn default() -> Self {
        Self { _priv: () }
    }
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    let transport = build_transport(&local_key).await?;

    let mut swarm = SwarmBuilder::with_tokio_executor(transport, MyBehaviour::default(), local_peer_id).build();

    let addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    swarm.listen_on(addr)?;

    loop {
        match swarm.select_next_some().await {
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
    local_key: &identity::Keypair,
) -> Result<Boxed<(PeerId, StreamMuxerBox)>, Box<dyn Error>> {
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default());
    let ws_transport = websocket::WsConfig::new(tcp::tokio::Transport::new(tcp::Config::default()));

    let noise_config = noise::Config::new(local_key)?;

    let transport = tcp_transport
        .or_transport(ws_transport)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(MplexConfig::new())
        .timeout(Duration::from_secs(20))
        .boxed();

    Ok(transport)
}
