use libp2p::{
    core::{
        upgrade::{DeniedUpgrade},
    },
    identity,
    noise,
    SwarmBuilder,
    swarm::{
        handler::ConnectionHandler,
        NetworkBehaviour,
        ConnectionHandlerEvent,
        SubstreamProtocol,
        SwarmEvent,
    },
    tcp,
    websocket,
    Multiaddr,
    PeerId,
};
use libp2p_mplex::MplexConfig;
use std::error::Error;
use std::task::{Context, Poll};
use void::Void;

#[derive(NetworkBehaviour)]
#[behaviour(connection_handler = "MyHandler", out_event = "MyBehaviourEvent")]
pub struct MyBehaviour;

#[derive(Debug)]
pub enum MyBehaviourEvent {}

impl Default for MyBehaviour {
    fn default() -> Self {
        Self
    }
}

#[derive(Default)]
pub struct MyHandler;

impl ConnectionHandler for MyHandler {
    type FromBehaviour = Void;
    type ToBehaviour = Void;
    type InboundProtocol = DeniedUpgrade;
    type OutboundProtocol = DeniedUpgrade;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(DeniedUpgrade, ())
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
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
            Self::ToBehaviour,
        >,
    > {
        Poll::Pending
    }

    fn on_connection_event(
        &mut self,
        _event: libp2p::swarm::handler::ConnectionEvent<
            Self::InboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
        >,
    ) {
    }
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, || {
            MplexConfig::new()
        })?
        .with_websocket(noise::Config::new, || {
            MplexConfig::new()
        })?
        .with_behaviour(|_| MyBehaviour::default())?
        .with_swarm_config(|_| Default::default())
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
