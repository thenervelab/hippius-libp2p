use libp2p::Swarm;
use std::{
    collections::HashMap,
    error::Error,
    sync::Arc,
    time::Duration,
    fs,
    path::Path,
};
use futures::{StreamExt, SinkExt};
use libp2p::{
    gossipsub, mdns, noise,
    swarm::{NetworkBehaviour, SwarmEvent, Config as SwarmConfig},
    tcp, yamux, PeerId, Multiaddr, dns,
    identity::{self, Keypair},
    core::{
        transport::Transport,
        upgrade,
    },
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
};
use tokio_tungstenite::tungstenite::Message;
use tracing_subscriber::EnvFilter;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use clap::Parser;

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

// Structure to hold room information
#[derive(Default, Clone)]
struct Room {
    peers: HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>,
    document_state: Vec<u8>,
    encrypted: bool,
    last_updated: u64,
}

#[derive(Clone, Serialize, Deserialize)]
struct RoomState {
    document_state: Vec<u8>,
    encrypted: bool,
    last_updated: u64,
    peer_count: usize,
}

impl Room {
    fn to_state(&self) -> RoomState {
        RoomState {
            document_state: self.document_state.clone(),
            encrypted: self.encrypted,
            last_updated: self.last_updated,
            peer_count: self.peers.len(),
        }
    }
}

type RoomMap = Arc<RwLock<HashMap<String, Room>>>;
type PeerMap = Arc<RwLock<HashMap<String, (String, tokio::sync::mpsc::UnboundedSender<Message>)>>>;

// We create a custom network behaviour that combines Gossipsub and Mdns
#[derive(NetworkBehaviour)]
struct ServerBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

struct P2pServer {
    swarm: libp2p::Swarm<ServerBehaviour>,
    room_map: RoomMap,
    peer_map: PeerMap,
    topic: gossipsub::IdentTopic,
}

#[derive(Serialize, Deserialize)]
enum ServerMessage {
    RoomUpdate {
        room_id: String,
        room: RoomState,
        timestamp: u64,
    },
}

impl P2pServer {
    fn load_or_create_identity(is_bootnode: bool) -> Result<Keypair, Box<dyn Error>> {
        let key_file = if is_bootnode {
            "bootnode_key.json"
        } else {
            "node_key.json"
        };

        if Path::new(key_file).exists() {
            // Load existing keypair
            let key_json = fs::read_to_string(key_file)?;
            let key_bytes: Vec<u8> = serde_json::from_str(&key_json)?;
            Ok(Keypair::from_protobuf_encoding(&key_bytes)?)
        } else {
            // Generate new keypair
            let keypair = identity::Keypair::generate_ed25519();
            let key_bytes = keypair.to_protobuf_encoding()?;
            let key_json = serde_json::to_string(&key_bytes)?;
            fs::write(key_file, key_json)?;
            Ok(keypair)
        }
    }

    async fn new(room_map: RoomMap, peer_map: PeerMap) -> Result<Self, Box<dyn Error>> {
        // Parse command line arguments first to know if we're a bootnode
        let args = Args::parse();
        
        let id_keys = Self::load_or_create_identity(args.bootnode)?;
        let peer_id = PeerId::from(id_keys.public());
        println!("Local peer id: {}", peer_id);

        // Create a transport with TCP and DNS support
        let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default())
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(&id_keys)?)
            .multiplex(yamux::Config::default())
            .boxed();

        // Add DNS support
        let transport = dns::tokio::Transport::system(tcp_transport)?.boxed();

        // Set up gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .expect("Valid config");

        // Create a Gossipsub topic
        let topic = gossipsub::IdentTopic::new("room-updates");

        let mut behaviour = ServerBehaviour {
            gossipsub: gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(id_keys.clone()),
                gossipsub_config,
            )?,
            mdns: mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)?,
        };

        behaviour.gossipsub.subscribe(&topic)?;

        let mut swarm = Swarm::new(
            transport,
            behaviour,
            peer_id,
            SwarmConfig::with_tokio_executor(),
        );

        if args.bootnode {
            // Listen on all interfaces if running as bootnode
            swarm.listen_on("/ip4/0.0.0.0/tcp/58455".parse()?)?;
            println!("Running as bootnode");
        } else {
            // Listen on localhost for regular nodes
            swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?;

            // Connect to bootnode if specified
            if let Some(addr) = args.bootnode_addr {
                println!("Connecting to bootnode: {}", addr);
                let multiaddr: Multiaddr = addr.parse().expect("Invalid multiaddr");
                swarm.dial(multiaddr).expect("Failed to dial bootnode");
            }
        }

        Ok(Self {
            swarm,
            room_map,
            peer_map,
            topic,
        })
    }

    async fn broadcast_room_update(&mut self, room_id: String, room: Room) {
        let message = ServerMessage::RoomUpdate {
            room_id: room_id.clone(),
            room: room.to_state(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        if let Ok(json) = serde_json::to_string(&message) {
            if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(
                self.topic.clone(),
                json.as_bytes(),
            ) {
                eprintln!("Publishing error: {e:?}");
            }
        }
    }

    async fn start(&mut self) {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(ServerBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _addr) in list {
                        println!("mDNS discovered a new peer: {peer_id}");
                        self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                }
                SwarmEvent::Behaviour(ServerBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _addr) in list {
                        println!("mDNS discover peer has expired: {peer_id}");
                        self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                }
                SwarmEvent::Behaviour(ServerBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) => {
                    if let Ok(msg) = serde_json::from_slice::<ServerMessage>(&message.data) {
                        match msg {
                            ServerMessage::RoomUpdate { room_id, room, timestamp } => {
                                let mut rooms = self.room_map.write().await;
                                let should_update = if let Some(existing) = rooms.get(&room_id) {
                                    timestamp > existing.last_updated
                                } else {
                                    true
                                };

                                if should_update {
                                    let mut new_room = Room {
                                        peers: HashMap::new(),
                                        document_state: room.document_state.clone(),
                                        encrypted: room.encrypted,
                                        last_updated: timestamp,
                                    };
                                    rooms.insert(room_id.clone(), new_room);
                                    println!(
                                        "Room {} updated with id: {} from peer: {}",
                                        room_id, id, peer_id
                                    );
                                }
                            }
                        }
                    }
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Local node is listening on {address}");
                }
                _ => {}
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct JoinPayload {
    user_id: u64,
    user_color: String,
    room_id: String,
    encrypted_data: Option<String>, // Base64 encoded encrypted data
}

#[derive(Serialize, Deserialize, Clone)]
struct SyncUpdatePayload {
    update: Vec<u8>,
    room_id: String,
    encrypted_data: Option<String>, // Base64 encoded encrypted data
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
enum SignalingMessage {
    Join(JoinPayload),
    SyncUpdate(SyncUpdatePayload),
    LeaveRoom { room_id: String },
    GetRooms,
    RoomList { rooms: Vec<RoomInfo> },
}

#[derive(Serialize, Deserialize, Clone)]
struct RoomInfo {
    room_id: String,
    peer_count: usize,
    encrypted: bool,
}

async fn handle_connection(
    room_map: RoomMap,
    peer_map: PeerMap,
    raw_stream: TcpStream,
    addr: std::net::SocketAddr
) {
    println!("New WebSocket connection: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during WebSocket handshake");
    
    let (mut write, mut read) = ws_stream.split();
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let peer_id = Uuid::new_v4().to_string();
    
    println!("Peer {} connected", peer_id);

    // Handle incoming messages
    let read_future = {
        let peer_id = peer_id.clone();
        let peer_map = peer_map.clone();
        let room_map = room_map.clone();

        async move {
            while let Some(result) = read.next().await {
                let msg = match result {
                    Ok(msg) => msg,
                    Err(e) => {
                        println!("Error receiving message from {}: {}", peer_id, e);
                        break;
                    }
                };

                if let Ok(text) = msg.to_text() {
                    if let Ok(signal_msg) = serde_json::from_str::<SignalingMessage>(text) {
                        match signal_msg {
                            SignalingMessage::Join(payload) => {
                                let peer_id = Uuid::new_v4().to_string();
                                let mut peers = peer_map.write().await;
                                peers.insert(peer_id.clone(), (payload.room_id.clone(), sender.clone()));

                                // Add peer to room
                                {
                                    let mut rooms = room_map.write().await;
                                    let room = rooms.entry(payload.room_id.clone()).or_default();
                                    room.peers.insert(peer_id.clone(), sender.clone());
                                    if let Some(_) = payload.encrypted_data {
                                        room.encrypted = true;
                                    }

                                    // Notify all peers in the room about the new peer
                                    for (id, peer_tx) in room.peers.iter() {
                                        if id != &peer_id {
                                            peer_tx.send(Message::Text(
                                                serde_json::to_string(&SignalingMessage::Join(payload.clone())).unwrap(),
                                            )).unwrap_or_default();
                                        }
                                    }

                                    // Send current document state to new peer
                                    if !room.document_state.is_empty() {
                                        sender.send(Message::Text(
                                            serde_json::to_string(&SignalingMessage::SyncUpdate(SyncUpdatePayload {
                                                update: room.document_state.clone(),
                                                room_id: payload.room_id.clone(),
                                                encrypted_data: if room.encrypted { Some(BASE64.encode(&room.document_state)) } else { None },
                                            })).unwrap(),
                                        )).unwrap_or_default();
                                    }
                                }
                            }
                            SignalingMessage::SyncUpdate(payload) => {
                                let room_id = payload.room_id.clone();
                                
                                // Update room's document state
                                let mut rooms = room_map.write().await;
                                if let Some(room) = rooms.get_mut(&room_id) {
                                    if let Some(ref encrypted_data) = payload.encrypted_data {
                                        room.document_state = BASE64.decode(encrypted_data).unwrap();
                                    } else {
                                        room.document_state = payload.update.clone();
                                    }
                                    
                                    // Broadcast update to all peers in the room
                                    for (id, peer_tx) in room.peers.iter() {
                                        if id != &peer_id {
                                            peer_tx.send(Message::Text(text.to_string())).unwrap_or_default();
                                        }
                                    }
                                }
                            }
                            SignalingMessage::LeaveRoom { room_id } => {
                                remove_peer_from_room(&peer_id, &room_id, &room_map, &peer_map).await;
                            }
                            SignalingMessage::GetRooms => {
                                let rooms = room_map.read().await;
                                let room_list: Vec<RoomInfo> = rooms.iter()
                                    .map(|(room_id, room)| RoomInfo {
                                        room_id: room_id.clone(),
                                        peer_count: room.peers.len(),
                                        encrypted: room.encrypted,
                                    })
                                    .collect();
                                
                                let _ = sender.send(Message::Text(
                                    serde_json::to_string(&SignalingMessage::RoomList { rooms: room_list }).unwrap(),
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    };

    // Handle outgoing messages
    let write_future = async {
        while let Some(message) = receiver.recv().await {
            write.send(message).await.unwrap_or_else(|e| {
                println!("Error sending message to {}: {}", peer_id, e);
            });
        }
    };

    // Run both tasks concurrently
    tokio::select! {
        _ = read_future => {},
        _ = write_future => {},
    }

    // Clean up on disconnect
    if let Some((room_id, _)) = peer_map.read().await.get(&peer_id) {
        remove_peer_from_room(&peer_id, room_id, &room_map, &peer_map).await;
    }
    println!("Peer {} disconnected", peer_id);
}

async fn remove_peer_from_room(peer_id: &str, room_id: &str, room_map: &RoomMap, peer_map: &PeerMap) {
    let mut rooms = room_map.write().await;
    if let Some(room) = rooms.get_mut(room_id) {
        room.peers.remove(peer_id);
        
        // Remove room if empty
        if room.peers.is_empty() {
            rooms.remove(room_id);
        }
    }
    peer_map.write().await.remove(peer_id);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let args = Args::parse();
    let room_map = Arc::new(RwLock::new(HashMap::new()));
    let peer_map = Arc::new(RwLock::new(HashMap::new()));

    // Create and start P2P server
    let mut p2p_server = P2pServer::new(room_map.clone(), peer_map.clone()).await?;
    let p2p = tokio::spawn(async move {
        p2p_server.start().await;
    });

    // Start WebSocket server if not running as bootnode
    if !args.bootnode {
        let addr = format!("127.0.0.1:{}", args.port);
        let listener = TcpListener::bind(&addr).await?;
        println!("WebSocket server listening on: {}", addr);

        let ws_server = tokio::spawn(async move {
            while let Ok((stream, addr)) = listener.accept().await {
                let room_map = Arc::clone(&room_map);
                let peer_map = Arc::clone(&peer_map);
                
                tokio::spawn(async move {
                    handle_connection(room_map, peer_map, stream, addr).await;
                });
            }
        });

        // Wait for both servers
        tokio::try_join!(p2p, ws_server)?;
    } else {
        // Wait only for P2P server if running as bootnode
        p2p.await?;
    }

    Ok(())
}
