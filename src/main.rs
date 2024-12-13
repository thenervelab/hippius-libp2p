use libp2p::{
    core::{
        muxing::StreamMuxerBox,
        transport::{Boxed, Transport},
        upgrade::{self, DeniedUpgrade},
    },
    identity,
    noise,
    SwarmBuilder,
    swarm::{
        ConnectionEvent, ConnectionId, NetworkBehaviour, ConnectionHandler,
        ConnectionHandlerEvent, SubstreamProtocol, SwarmEvent,
    },
    tcp,
    websocket,
    webrtc,
    Multiaddr, PeerId,
    futures::StreamExt,
};
use libp2p_mplex::MplexConfig;
use std::error::Error;
use std::time::Duration;
use std::task::{Context, Poll};
use void::Void;

#[derive(NetworkBehaviour, Default)]
#[behaviour(connection_handler = "MyHandler", out_event = "MyBehaviourEvent")]
pub struct MyBehaviour {
}

#[derive(Debug)]
pub enum MyBehaviourEvent {}

#[derive(Default)]
pub struct MyHandler;

impl ConnectionHandler for MyHandler {
    type InEvent = Void;
    type OutEvent = Void;
    type InboundProtocol = DeniedUpgrade;
    type OutboundProtocol = DeniedUpgrade;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(DeniedUpgrade, ())
    }

    fn on_behaviour_event(&mut self, event: Self::InEvent) {
        void::unreachable(event)
    }

    fn connection_keep_alive(&self) -> bool {
        true
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<
        ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::OutEvent,
        >,
    > {
        Poll::Pending
    }

    fn on_connection_event(
        &mut self,
        event: ConnectionEvent<
            Self::InboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
        >,
    ) {
        match event {
            _ => {}
        }
    }
}


async fn build_transport() -> Result<Boxed<()>, Box<dyn Error>> {
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default());
    let ws_transport = websocket::tokio::Transport::new(websocket::Config::default(), tcp::tokio::Transport::new(tcp::Config::default()));
    let webrtc_transport = webrtc::tokio::Transport::new(webrtc::Config::default());

    let noise_config = noise::Config::new();
    let mplex_config = MplexConfig::new();

    let transport = tcp_transport
        .or_transport(ws_transport)
        .or_transport(webrtc_transport)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(mplex_config)
        .boxed();

    Ok(transport)
}


#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    let transport = build_transport().await?;

    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_other_transport(transport)
        .with_behaviour(|_| MyBehaviour::default())?
        .build();

    let addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    swarm.listen_on(addr)?;

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
