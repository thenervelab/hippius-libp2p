use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use warp::{
    ws::{Message, WebSocket},
    Filter,
};

type PeerId = String;
type PeerMap = Arc<RwLock<HashMap<PeerId, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
enum SignalingMessage {
    Register { peer_id: String },
    Offer { from: String, to: String, sdp: String },
    Answer { from: String, to: String, sdp: String },
    IceCandidate { from: String, to: String, candidate: String },
}

pub async fn start_signaling_server(port: u16) {
    let peer_map = Arc::new(RwLock::new(HashMap::new()));

    let peer_map = warp::any().map(move || peer_map.clone());

    let signaling = warp::path("signal")
        .and(warp::ws())
        .and(peer_map)
        .map(|ws: warp::ws::Ws, peer_map| {
            ws.on_upgrade(move |socket| handle_connection(socket, peer_map))
        });

    println!("Starting WebRTC signaling server on port {}", port);
    warp::serve(signaling).run(([0, 0, 0, 0], port)).await;
}

async fn handle_connection(ws: WebSocket, peer_map: PeerMap) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();
    
    let mut rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
    
    let peer_id = Arc::new(RwLock::new(String::new()));
    let peer_id_clone = peer_id.clone();

    // Forward messages from rx to websocket
    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            if let Ok(msg) = message {
                if let Err(e) = ws_tx.send(msg).await {
                    eprintln!("Failed to send WebSocket message: {}", e);
                    break;
                }
            }
        }
    });

    // Handle incoming WebSocket messages
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => {
                if let Ok(text) = msg.to_str() {
                    if let Ok(signal_msg) = serde_json::from_str::<SignalingMessage>(text) {
                        match signal_msg {
                            SignalingMessage::Register { peer_id: id } => {
                                let mut peer_id = peer_id_clone.write().await;
                                *peer_id = id.clone();
                                peer_map.write().await.insert(id, tx.clone());
                                println!("Peer registered: {}", peer_id);
                            }
                            SignalingMessage::Offer { from, to, sdp } => {
                                if let Some(peer_tx) = peer_map.read().await.get(&to) {
                                    let msg = SignalingMessage::Offer {
                                        from,
                                        to,
                                        sdp,
                                    };
                                    let _ = peer_tx.send(Ok(Message::text(
                                        serde_json::to_string(&msg).unwrap(),
                                    )));
                                }
                            }
                            SignalingMessage::Answer { from, to, sdp } => {
                                if let Some(peer_tx) = peer_map.read().await.get(&to) {
                                    let msg = SignalingMessage::Answer {
                                        from,
                                        to,
                                        sdp,
                                    };
                                    let _ = peer_tx.send(Ok(Message::text(
                                        serde_json::to_string(&msg).unwrap(),
                                    )));
                                }
                            }
                            SignalingMessage::IceCandidate { from, to, candidate } => {
                                if let Some(peer_tx) = peer_map.read().await.get(&to) {
                                    let msg = SignalingMessage::IceCandidate {
                                        from,
                                        to,
                                        candidate,
                                    };
                                    let _ = peer_tx.send(Ok(Message::text(
                                        serde_json::to_string(&msg).unwrap(),
                                    )));
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
        }
    }

    // Remove peer from map when connection closes
    let peer_id = peer_id_clone.read().await;
    if !peer_id.is_empty() {
        peer_map.write().await.remove(&*peer_id);
        println!("Peer disconnected: {}", peer_id);
    }
}
