use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use libp2p::SwarmBuilder;
use libp2p::{
    core::muxing::StreamMuxerBox,
    core::{transport::{OrTransport, Transport}, upgrade},
    gossipsub::{self},
    identity,
    mdns::{self, tokio::Behaviour as MdnsBehaviour},
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm,
};
use libp2p_webrtc::tokio::certificate::Certificate;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::{collections::HashMap, error::Error as StdError, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use tracing_subscriber::EnvFilter;

// Custom result type for our application logic
type AppResult<T> = std::result::Result<T, Box<dyn StdError + Send + Sync + 'static>>;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// WebSocket port to listen on
    #[arg(long, default_value = "8083")]
    port: u16,

    /// Run as a bootnode
    #[arg(long)]
    bootnode: bool,

    /// Bootnode address to connect to (e.g., "/ip4/127.0.0.1/tcp/58455/p2p/PEER_ID")
    #[arg(long)]
    bootnode_addr: Option<String>,
}

type PeerMap = Arc<RwLock<HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>>>;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "ServerBehaviourEvent")]
struct ServerBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: MdnsBehaviour,
}

#[derive(Debug)]
enum ServerBehaviourEvent {
    Gossipsub(gossipsub::Event),
    Mdns(mdns::Event),
}

impl From<gossipsub::Event> for ServerBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        ServerBehaviourEvent::Gossipsub(event)
    }
}

impl From<mdns::Event> for ServerBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        ServerBehaviourEvent::Mdns(event)
    }
}

struct P2pServer {
    swarm: Swarm<ServerBehaviour>,
    peer_map: PeerMap,
}

#[derive(Serialize, Deserialize)]
enum ServerMessage {
    PeerMessage {
        from_peer: String,
        message: Vec<u8>,
        timestamp: u64,
    },
}

impl P2pServer {
    async fn load_or_create_identity(is_bootnode: bool) -> AppResult<identity::Keypair> {
        let key_file = if is_bootnode {
            "bootnode.key"
        } else {
            "node.key"
        };

        if let Ok(bytes) = std::fs::read(key_file) {
            return Ok(identity::Keypair::from_protobuf_encoding(&bytes)?);
        }

        let keypair = identity::Keypair::generate_ed25519();
        std::fs::write(key_file, keypair.to_protobuf_encoding()?)?;
        Ok(keypair)
    }

    async fn new(peer_map: PeerMap) -> AppResult<Self> {
        let args = Args::parse();
        let local_key = Self::load_or_create_identity(args.bootnode).await?;
        let local_peer_id = PeerId::from(local_key.public());
        println!("Local peer id: {local_peer_id}");

        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(message_id_fn)
            .build()
            .expect("Valid config");

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )?;

        let behaviour = ServerBehaviour {
            gossipsub,
            mdns: mdns::Behaviour::new(mdns::Config::default(), local_peer_id)?,
        };

        // Setup TCP transport with noise encryption and yamux multiplexing
        let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default())
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(&local_key).unwrap())
            .multiplex(yamux::Config::default())
            .boxed();

        // Create and configure WebRTC transport
        let cert = Certificate::generate(&mut thread_rng())?;
        let webrtc_transport = libp2p_webrtc::tokio::Transport::new(local_key.clone(), cert)
            .map(|(_, conn)| ((), StreamMuxerBox::new(conn)))
            .boxed();

        // Combine transports using OrTransport
        let transport = OrTransport::new(tcp_transport, webrtc_transport);

        let swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
            .with_tokio()
            .build();

        // Listen on TCP
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        Ok(Self { swarm, peer_map })
    }

    async fn broadcast_message(&mut self, from_peer: String, message: Vec<u8>) -> AppResult<()> {
        let msg = ServerMessage::PeerMessage {
            from_peer,
            message,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let encoded = serde_json::to_vec(&msg)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(gossipsub::IdentTopic::new("global"), encoded)?;
        Ok(())
    }

    async fn start(mut self) -> AppResult<()> {
        // Subscribe to the global topic
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&gossipsub::IdentTopic::new("global"))?;

        // Connect to bootnode if specified
        let args = Args::parse();
        if let Some(addr) = args.bootnode_addr {
            let remote: Multiaddr = addr.parse()?;
            self.swarm.dial(remote)?;
            println!("Dialed bootnode: {}", addr);
        }

        loop {
            tokio::select! {
                event = self.swarm.next() => match event {
                    Some(SwarmEvent::ConnectionEstablished { peer_id, .. }) => {
                        println!("Connection established to {}", peer_id);
                    }
                    Some(SwarmEvent::ConnectionClosed { peer_id, .. }) => {
                        println!("Connection closed to {}", peer_id);
                    }
                    Some(SwarmEvent::NewListenAddr { listener_id: _, address }) => {
                        println!("Local node is listening on {address}");
                    }
                    Some(SwarmEvent::Behaviour(ServerBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    }))) => {
                        if let Ok(msg) = serde_json::from_slice::<ServerMessage>(&message.data) {
                            match msg {
                                ServerMessage::PeerMessage { from_peer, message, timestamp: _ } => {
                                    // Forward message to all connected WebSocket clients
                                    let peers = self.peer_map.read().await;
                                    for (_, sender) in peers.iter() {
                                        sender.send(Message::Binary(message.clone())).unwrap_or_default();
                                    }
                                    println!(
                                        "Message from peer {} forwarded, id: {} from: {}",
                                        from_peer, id, peer_id
                                    );
                                }
                            }
                        }
                    }
                    Some(SwarmEvent::Behaviour(ServerBehaviourEvent::Mdns(mdns::Event::Discovered(list)))) => {
                        for (peer_id, _addr) in list {
                            println!("mDNS discovered a new peer: {peer_id}");
                            self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                    }
                    Some(SwarmEvent::Behaviour(ServerBehaviourEvent::Mdns(mdns::Event::Expired(list)))) => {
                        for (peer_id, _addr) in list {
                            println!("mDNS peer has expired: {peer_id}");
                            self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> AppResult<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let peer_map = Arc::new(RwLock::new(HashMap::new()));

    // Start P2P server
    let p2p_server = P2pServer::new(peer_map.clone()).await?;
    let p2p_handle = tokio::spawn(async move {
        if let Err(e) = p2p_server.start().await {
            eprintln!("P2P server error: {}", e);
        }
    });

    // Start WebSocket server
    let addr = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync>)?;
    println!("WebSocket server listening on: {}", addr);

    let server_handle = tokio::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            let peer_map = peer_map.clone();
            tokio::spawn(async move {
                handle_connection(peer_map, stream, addr).await;
            });
        }
    });

    // Wait for either server to finish
    tokio::select! {
        res = p2p_handle => res.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync>),
        res = server_handle => res.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync>),
    }
}

async fn handle_connection(
    peer_map: PeerMap,
    raw_stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
) {
    println!("New WebSocket connection: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during WebSocket handshake");

    let (mut write, mut read) = ws_stream.split();
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let peer_id = uuid::Uuid::new_v4().to_string();
    println!("Peer {} connected", peer_id);

    // Add peer to the map
    peer_map
        .write()
        .await
        .insert(peer_id.clone(), sender.clone());

    let read_future = {
        let peer_id = peer_id.clone();
        let peer_map = peer_map.clone();

        async move {
            while let Some(result) = read.next().await {
                if result.is_err() {
                    break;
                }
            }
            peer_map.write().await.remove(&peer_id);
            println!("Peer {} disconnected", peer_id);
        }
    };

    let write_future = async move {
        while let Some(msg) = receiver.recv().await {
            if write.send(msg).await.is_err() {
                break;
            }
        }
    };

    futures::future::join(read_future, write_future).await;
}
