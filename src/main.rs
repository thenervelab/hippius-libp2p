use futures_util::StreamExt;
use libp2p::{
    core::{
        transport::{Boxed, OrTransport, Transport},
        upgrade,
    },
    gossipsub::{self, IdentTopic},
    identity::{self, Keypair},
    mdns::{self, tokio::Behaviour as MdnsBehaviour},
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, websocket, yamux, PeerId, Swarm,
};
use rand::{rngs::ThreadRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error::Error as StdError,
    fs,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::AsyncBufReadExt,
    sync::RwLock,
};

type PeerMap = Arc<RwLock<HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>>>;

mod monitoring;
mod metrics_server;

use monitoring::Monitoring;

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

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    PeerMessage { from_peer: String, message: Vec<u8> },
    Command { command: String, args: Vec<String> },
}

struct P2pServer {
    swarm: Swarm<ServerBehaviour>,
    peer_map: PeerMap,
    topics: HashMap<String, IdentTopic>,
    monitoring: Arc<Monitoring>,
}

impl P2pServer {
    async fn new(peer_map: PeerMap, is_bootnode: bool) -> std::result::Result<Self, Box<dyn StdError + Send + Sync + 'static>> {
        // Create data directory if it doesn't exist
        let data_dir = if is_bootnode {
            PathBuf::from("data/bootnode")
        } else {
            PathBuf::from("data/node")
        };
        fs::create_dir_all(&data_dir)?;

        // Try to load or create key pair
        let key_file = data_dir.join("peer_id.key");
        let local_key = if key_file.exists() {
            // Load existing key
            let key_bytes = fs::read(&key_file)?;
            let key_str = String::from_utf8(key_bytes)?;
            let key_bytes = bs58::decode(key_str).into_vec()?;
            Keypair::from_protobuf_encoding(&key_bytes)?
        } else {
            // Generate new key
            let local_key = Keypair::generate_ed25519();
            let key_bytes = local_key.to_protobuf_encoding()?;
            let key_str = bs58::encode(key_bytes).into_string();
            fs::write(&key_file, key_str)?;
            local_key
        };

        let local_peer_id = PeerId::from(local_key.public());

        // Set up gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .expect("Valid config");

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )?;

        // Create behaviour
        let behaviour = ServerBehaviour {
            gossipsub,
            mdns: mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?,
        };

        // Set up TCP transport
        let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default())
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(&local_key)?)
            .multiplex(yamux::Config::default());

        // Set up WebSocket transport
        let ws_transport = websocket::WsConfig::new(tcp::tokio::Transport::new(tcp::Config::default()))
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(&local_key)?)
            .multiplex(yamux::Config::default());

        // Combine TCP and WebSocket transports
        let transport: Boxed<(PeerId, libp2p::core::muxing::StreamMuxerBox)> = OrTransport::new(tcp_transport, ws_transport)
            .map(|either_output, _| {
                match either_output {
                    futures_util::future::Either::Left((peer_id, muxer)) => 
                        (peer_id, libp2p::core::muxing::StreamMuxerBox::new(muxer)),
                    futures_util::future::Either::Right((peer_id, muxer)) => 
                        (peer_id, libp2p::core::muxing::StreamMuxerBox::new(muxer)),
                }
            })
            .boxed();

        // Create swarm with tokio executor
        let mut swarm = Swarm::new(
            transport,
            behaviour,
            local_peer_id,
            libp2p::swarm::Config::with_tokio_executor(),
        );

        // Listen on all supported protocols
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse::<libp2p::Multiaddr>()?)?;
        swarm.listen_on("/ip4/0.0.0.0/tcp/0/ws".parse::<libp2p::Multiaddr>()?)?;

        // If not a bootnode, connect to bootstrap nodes
        if !is_bootnode {
            let bootstrap_addresses = vec![
                "/ip4/127.0.0.1/tcp/4001".parse::<libp2p::Multiaddr>()?,
                "/ip4/127.0.0.1/tcp/4001/ws".parse::<libp2p::Multiaddr>()?,
            ];

            for addr in bootstrap_addresses {
                swarm.dial(addr)?;
            }
        }

        let monitoring = Arc::new(Monitoring::new());

        Ok(Self { 
            swarm, 
            peer_map,
            topics: HashMap::new(),
            monitoring,
        })
    }

    async fn broadcast_message(&mut self, from_peer: String, message: Vec<u8>) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        let msg = Message::PeerMessage {
            from_peer,
            message,
        };
        let msg_bytes = serde_json::to_vec(&msg)?;

        let topics: Vec<_> = self.swarm.behaviour().gossipsub.topics().cloned().collect();
        for topic in topics {
            self.swarm
                .behaviour_mut()
                .gossipsub
                .publish(topic, msg_bytes.clone())?;
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: &str, args: &[String]) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        match command {
            "/create-topic" | "/join-topic" if !args.is_empty() => {
                let topic_name = &args[0];
                let topic = IdentTopic::new(topic_name);
                self.topics.insert(topic_name.clone(), topic.clone());
                self.swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
                println!("Subscribed to topic: {}", topic_name);
            }
            "/send" if args.len() >= 2 => {
                let topic_name = &args[0];
                let message = &args[1];
                if let Some(topic) = self.topics.get(topic_name) {
                    self.broadcast_message_to_topic(topic.clone(), message.as_bytes().to_vec()).await?;
                } else {
                    println!("Not subscribed to topic: {}", topic_name);
                }
            }
            _ => {
                println!("Unknown command or invalid arguments");
                println!("Available commands:");
                println!("  /create-topic <topic>    - Create and join a new topic");
                println!("  /join-topic <topic>      - Join an existing topic");
                println!("  /send <topic> <message>  - Send a message to a topic");
            }
        }
        Ok(())
    }

    async fn broadcast_message_to_topic(&mut self, topic: IdentTopic, message: Vec<u8>) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, message)?;
        Ok(())
    }

    async fn start(&mut self) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        // Start metrics server
        let monitoring = self.monitoring.clone();
        tokio::spawn(async move {
            if let Err(e) = metrics_server::start_metrics_server(monitoring).await {
                eprintln!("Metrics server error: {}", e);
            }
        });

        let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

        loop {
            tokio::select! {
                line = stdin.next_line() => {
                    if let Ok(Some(line)) = line {
                        if line.starts_with('/') {
                            let parts: Vec<String> = line.split_whitespace().map(String::from).collect();
                            if !parts.is_empty() {
                                let command = &parts[0];
                                let args = &parts[1..];
                                if let Err(e) = self.handle_command(command, args.to_vec().as_slice()).await {
                                    println!("Error handling command: {}", e);
                                }
                            }
                        }
                    }
                }
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(ServerBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                        for (peer_id, _) in list {
                            self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                    }
                    SwarmEvent::Behaviour(ServerBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                        for (peer_id, _) in list {
                            self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                        }
                    }
                    SwarmEvent::Behaviour(ServerBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    })) => {
                        self.monitoring.record_message_received(&peer_id, message.data.len() as u64).await;
                        println!(
                            "Got message: {} with id: {} from peer: {:?}",
                            String::from_utf8_lossy(&message.data),
                            id,
                            peer_id
                        );
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on {:?}", address);
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        self.monitoring.record_peer_connected(peer_id, "direct").await;
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        self.monitoring.record_peer_disconnected(&peer_id).await;
                    }
                    _ => {}
                }
            }
        }
    }

    fn peer_id(&self) -> PeerId {
        *self.swarm.local_peer_id()
    }
}

mod signaling;
mod web_server;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run mode: 'all' for all servers, 'bootnode' for bootnode only, 'node' for regular node, 'signaling' for signaling+web servers
    #[arg(long, default_value = "node")]
    mode: String,

    /// Port for signaling server
    #[arg(long, default_value = "8001")]
    signaling_port: u16,

    /// Port for web server
    #[arg(long, default_value = "3000")]
    web_port: u16,

    /// Port for bootnode
    #[arg(long, default_value = "4002")]
    bootnode_port: u16,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    
    match args.mode.as_str() {
        "all" => {
            println!("Starting all servers...");
            println!("Web server: http://localhost:{}", args.web_port);
            println!("Signaling server: ws://localhost:{}", args.signaling_port);
            println!("Metrics server: http://localhost:9091");
            
            // Start web server, signaling server, and bootnode
            let peer_map = Arc::new(RwLock::new(HashMap::new()));
            let mut bootnode = P2pServer::new(peer_map.clone(), true).await?;
            println!("Bootnode: /ip4/127.0.0.1/tcp/{}", args.bootnode_port);
            println!("Bootnode PeerID: {}", bootnode.peer_id());
            
            tokio::join!(
                web_server::start_web_server(args.web_port),
                signaling::start_signaling_server(args.signaling_port),
                bootnode.start()
            );
        }
        "signaling" => {
            println!("Starting signaling and web servers...");
            println!("Web server: http://localhost:{}", args.web_port);
            println!("Signaling server: ws://localhost:{}", args.signaling_port);
            
            tokio::join!(
                web_server::start_web_server(args.web_port),
                signaling::start_signaling_server(args.signaling_port)
            );
        }
        "bootnode" => {
            println!("Starting bootnode...");
            let peer_map = Arc::new(RwLock::new(HashMap::new()));
            let mut server = P2pServer::new(peer_map.clone(), true).await?;
            println!("Bootnode: /ip4/127.0.0.1/tcp/{}", args.bootnode_port);
            println!("Bootnode PeerID: {}", server.peer_id());
            server.start().await?;
        }
        "node" => {
            println!("Starting regular node with signaling and web servers...");
            println!("Web server: http://localhost:{}", args.web_port);
            println!("Signaling server: ws://localhost:{}", args.signaling_port);
            
            let peer_map = Arc::new(RwLock::new(HashMap::new()));
            let mut server = P2pServer::new(peer_map.clone(), false).await?;
            println!("Node PeerID: {}", server.peer_id());
            
            tokio::join!(
                web_server::start_web_server(args.web_port),
                signaling::start_signaling_server(args.signaling_port),
                server.start()
            );
        }
        _ => {
            println!("Invalid mode. Available modes: all, signaling, bootnode, node");
            std::process::exit(1);
        }
    }

    Ok(())
}
