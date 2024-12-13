use libp2p::{
    core::{
        transport::{Boxed, Transport},
        upgrade,
    },
    identity,
    noise,
    swarm::{Swarm, SwarmBuilder, SwarmEvent, NetworkBehaviour, ConnectionHandler, ConnectionId, ConnectionEvent},
    tcp,
    websocket,
    Multiaddr, PeerId,
};
use libp2p_mplex::MplexConfig;
use std::error::Error;
use libp2p::webrtc;
use void::Void;

#[derive(NetworkBehaviour, Default)]
#[behaviour(out_event = "MyBehaviourEvent", event_process = false)]
struct MyBehaviour {
    #[behaviour(ignore)]
    _priv: (),
}

#[derive(Debug)]
enum MyBehaviourEvent {}

impl ConnectionHandler for MyBehaviour {
    type FromBehaviour = ();
    type ToBehaviour = Void;
    type InEvent = Void;
    type OutEvent = Void;

    fn on_connection_event(
        &mut self,
        _peer_id: PeerId,
        _connection_id: ConnectionId,
        _event: ConnectionEvent<Self::InEvent, Self::OutEvent>,
    ) {
    }
}

impl MyBehaviour {
    fn on_behaviour_event(&mut self, _event: MyBehaviourEvent) {
    }
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    let transport = build_transport(&local_key).await?;

    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_other_transport(move |_| transport)
        .with_behaviour(|_| MyBehaviour::default())?
        .build();

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
) -> Result<Boxed<(PeerId, libp2p::core::muxing::StreamMuxerBox)>, Box<dyn Error>> {
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default());
    let ws_transport = websocket::WsConfig::new(tcp::tokio::Transport::new(tcp::Config::default()));

    let noise_config = noise::Config::new(local_key);

    let transport = tcp_transport
        .or_transport(ws_transport)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise_config?)
        .multiplex(MplexConfig::new())
        .boxed();

    Ok(transport)
}
