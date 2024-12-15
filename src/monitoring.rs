use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;
use serde::Serialize;
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use libp2p::PeerId;
use sysinfo::{System, SystemExt, CpuExt, DiskExt};

#[derive(Debug, Clone, Serialize)]
pub struct NetworkStats {
    pub connected_peers: usize,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub uptime_secs: u64,
    pub peer_connections: HashMap<String, PeerStats>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeerStats {
    pub peer_id: String,
    pub connected_since: u64,  // Unix timestamp
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connection_type: String, // "direct", "stun", or "turn"
    pub latency_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemStats {
    pub cpu_usage: f32,  // Changed to f32 to match sysinfo
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub thread_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebSocketStats {
    pub active_connections: usize,
    pub total_connections: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
}

pub struct Monitoring {
    start_time: SystemTime,
    network_stats: Arc<RwLock<NetworkStats>>,
    system_stats: Arc<RwLock<SystemStats>>,
    websocket_stats: Arc<RwLock<WebSocketStats>>,
    prometheus_handle: Arc<PrometheusHandle>,
}

impl Monitoring {
    pub fn new() -> Self {
        // Initialize Prometheus metrics exporter
        let builder = PrometheusBuilder::new();
        let handle = builder
            .with_http_listener(([127, 0, 0, 1], 9091))
            .install_recorder()
            .expect("failed to install Prometheus recorder");

        let monitoring = Self {
            start_time: SystemTime::now(),
            network_stats: Arc::new(RwLock::new(NetworkStats {
                connected_peers: 0,
                messages_sent: 0,
                messages_received: 0,
                bytes_sent: 0,
                bytes_received: 0,
                uptime_secs: 0,
                peer_connections: HashMap::new(),
            })),
            system_stats: Arc::new(RwLock::new(SystemStats {
                cpu_usage: 0.0,
                memory_usage: 0.0,
                disk_usage: 0.0,
                thread_count: 0,
            })),
            websocket_stats: Arc::new(RwLock::new(WebSocketStats {
                active_connections: 0,
                total_connections: 0,
                messages_sent: 0,
                messages_received: 0,
            })),
            prometheus_handle: Arc::new(handle),
        };

        // Start background monitoring tasks
        monitoring.start_background_tasks();
        monitoring
    }

    pub fn get_prometheus_handle(&self) -> Arc<PrometheusHandle> {
        self.prometheus_handle.clone()
    }

    fn start_background_tasks(&self) {
        let network_stats = self.network_stats.clone();
        let system_stats = self.system_stats.clone();
        let websocket_stats = self.websocket_stats.clone();
        let start_time = self.start_time;

        // Update metrics every second
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;

                // Update network metrics
                let network = network_stats.read().await;
                gauge!("p2p_connected_peers", network.connected_peers as f64);
                counter!("p2p_messages_sent", network.messages_sent);
                counter!("p2p_messages_received", network.messages_received);
                counter!("p2p_bytes_sent", network.bytes_sent);
                counter!("p2p_bytes_received", network.bytes_received);

                // Update system metrics
                let system = system_stats.read().await;
                gauge!("system_cpu_usage", system.cpu_usage as f64);
                gauge!("system_memory_usage", system.memory_usage);
                gauge!("system_disk_usage", system.disk_usage);
                gauge!("system_thread_count", system.thread_count as f64);

                // Update WebSocket metrics
                let ws = websocket_stats.read().await;
                gauge!("ws_active_connections", ws.active_connections as f64);
                counter!("ws_total_connections", ws.total_connections);
                counter!("ws_messages_sent", ws.messages_sent);
                counter!("ws_messages_received", ws.messages_received);

                // Update uptime
                if let Ok(duration) = start_time.elapsed() {
                    gauge!("uptime_seconds", duration.as_secs() as f64);
                }
            }
        });

        // Update system stats every 5 seconds
        let system_stats = self.system_stats.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            let mut sys = System::new();
            loop {
                interval.tick().await;
                let mut stats = system_stats.write().await;

                // Update system metrics
                sys.refresh_all();

                stats.cpu_usage = sys.global_cpu_info().cpu_usage();
                stats.memory_usage = (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0;
                stats.disk_usage = sys
                    .disks()
                    .iter()
                    .map(|disk| (disk.total_space() - disk.available_space()) as f64)
                    .sum::<f64>();
                stats.thread_count = sys.processes().len();
            }
        });
    }

    pub async fn record_peer_connected(&self, peer_id: PeerId, connection_type: &str) {
        let mut stats = self.network_stats.write().await;
        stats.connected_peers += 1;
        stats.peer_connections.insert(
            peer_id.to_string(),
            PeerStats {
                peer_id: peer_id.to_string(),
                connected_since: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                messages_sent: 0,
                messages_received: 0,
                bytes_sent: 0,
                bytes_received: 0,
                connection_type: connection_type.to_string(),
                latency_ms: 0.0,
            },
        );
        gauge!("p2p_connected_peers", stats.connected_peers as f64);
    }

    pub async fn record_peer_disconnected(&self, peer_id: &PeerId) {
        let mut stats = self.network_stats.write().await;
        stats.connected_peers -= 1;
        stats.peer_connections.remove(&peer_id.to_string());
        gauge!("p2p_connected_peers", stats.connected_peers as f64);
    }

    pub async fn record_message_sent(&self, peer_id: &PeerId, bytes: u64) {
        let mut stats = self.network_stats.write().await;
        stats.messages_sent += 1;
        stats.bytes_sent += bytes;
        if let Some(peer_stats) = stats.peer_connections.get_mut(&peer_id.to_string()) {
            peer_stats.messages_sent += 1;
            peer_stats.bytes_sent += bytes;
        }
        counter!("p2p_messages_sent", 1);
        counter!("p2p_bytes_sent", bytes);
    }

    pub async fn record_message_received(&self, peer_id: &PeerId, bytes: u64) {
        let mut stats = self.network_stats.write().await;
        stats.messages_received += 1;
        stats.bytes_received += bytes;
        if let Some(peer_stats) = stats.peer_connections.get_mut(&peer_id.to_string()) {
            peer_stats.messages_received += 1;
            peer_stats.bytes_received += bytes;
        }
        counter!("p2p_messages_received", 1);
        counter!("p2p_bytes_received", bytes);
    }

    pub async fn record_websocket_connected(&self) {
        let mut stats = self.websocket_stats.write().await;
        stats.active_connections += 1;
        stats.total_connections += 1;
        gauge!("ws_active_connections", stats.active_connections as f64);
        counter!("ws_total_connections", 1);
    }

    pub async fn record_websocket_disconnected(&self) {
        let mut stats = self.websocket_stats.write().await;
        stats.active_connections -= 1;
        gauge!("ws_active_connections", stats.active_connections as f64);
    }

    pub async fn record_websocket_message(&self, is_outgoing: bool, _bytes: u64) {
        let mut stats = self.websocket_stats.write().await;
        if is_outgoing {
            stats.messages_sent += 1;
            counter!("ws_messages_sent", 1);
        } else {
            stats.messages_received += 1;
            counter!("ws_messages_received", 1);
        }
    }

    pub async fn get_all_stats(&self) -> (NetworkStats, SystemStats, WebSocketStats) {
        let network = self.network_stats.read().await.clone();
        let system = self.system_stats.read().await.clone();
        let websocket = self.websocket_stats.read().await.clone();
        (network, system, websocket)
    }
}
