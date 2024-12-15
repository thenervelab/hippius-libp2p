use axum::{
    routing::get,
    Router,
    response::Json,
    serve,
};
use std::net::SocketAddr;
use serde_json::{json, Value};
use crate::monitoring::Monitoring;
use std::sync::Arc;
use tokio::net::TcpListener;
use std::error::Error;

pub async fn start_metrics_server(monitoring: Arc<Monitoring>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let handle = monitoring.get_prometheus_handle();
    
    // Create router
    let app = Router::new()
        .route("/metrics", get(move || async move { 
            handle.render()
        }))
        .route("/stats", get(move || async move {
            let (network, system, websocket) = monitoring.get_all_stats().await;
            
            Json(json!({
                "network": {
                    "connected_peers": network.connected_peers,
                    "messages_sent": network.messages_sent,
                    "messages_received": network.messages_received,
                    "bytes_sent": network.bytes_sent,
                    "bytes_received": network.bytes_received,
                    "uptime_secs": network.uptime_secs,
                    "peer_connections": network.peer_connections
                },
                "system": {
                    "cpu_usage": system.cpu_usage,
                    "memory_usage": system.memory_usage,
                    "disk_usage": system.disk_usage,
                    "thread_count": system.thread_count
                },
                "websocket": {
                    "active_connections": websocket.active_connections,
                    "total_connections": websocket.total_connections,
                    "messages_sent": websocket.messages_sent,
                    "messages_received": websocket.messages_received
                }
            }))
        }));

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 9091));
    println!("Metrics server listening on http://127.0.0.1:9091");
    let listener = TcpListener::bind(addr).await?;
    serve(listener, app.into_make_service()).await?;

    Ok(())
}
