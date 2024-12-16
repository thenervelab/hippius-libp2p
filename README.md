# Hippius LibP2P Network

A robust peer-to-peer networking application built with Rust and LibP2P, supporting multiple transport protocols and distributed peer discovery. This project demonstrates how to build a flexible P2P network that can work across different network environments.

## Features

- **Multi-Transport Support**
  - TCP for direct connections
  - WebSocket for web-compatible connections
  - WebRTC for browser-based peer connections
  - Automatic transport negotiation and protocol upgrading
  - Noise protocol for encrypted communications

- **Peer Discovery**
  - MDNS for local network peer discovery
  - Gossipsub for efficient message broadcasting
  - Support for bootnode and regular node modes
  - Automatic peer discovery and connection management

- **Dynamic Topic Management**
  - Create and join topics on-the-fly
  - Topic-based message broadcasting
  - Multiple topic subscriptions per node
  - JSON-based message serialization

- **WebRTC Integration**
  - Browser-to-browser P2P connections
  - WebSocket signaling server for connection establishment
  - TURN server support for NAT traversal
  - Connection monitoring and statistics
  - Automatic fallback to TURN relay

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo (comes with Rust)
- Network connectivity (for peer discovery)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/thenervelab/hippius-libp2p.git
   cd hippius-libp2p
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

### Running

There are several ways to run the application:

1. Start everything (signaling server, web server, and bootnode) at once:
   ```bash
   cargo run -- --mode all
   ```
   This starts:
   - Web client server at http://localhost:3000
   - WebRTC signaling server at ws://localhost:8001
   - P2P bootnode at /ip4/127.0.0.1/tcp/4002

   The application will display:
   - URLs for accessing each server
   - The bootnode's peer ID (important for manual connections)
   - Each node's unique peer ID when started

   Then:
   1. Open http://localhost:3000 in multiple browser windows
   2. Watch as peers automatically discover each other
   3. Click on peers to establish WebRTC connections
   4. Start chatting peer-to-peer!

2. Start just the signaling and web servers:
   ```bash
   cargo run -- --mode signaling
   ```

3. Start a bootnode only:
   ```bash
   cargo run -- --mode bootnode
   ```

4. Start a regular node:
   ```bash
   # Start a regular node with web and signaling servers
   cargo run -- --mode node
   ```
   This starts:
   - A regular P2P node
   - Web client server (http://localhost:3000)
   - Signaling server (ws://localhost:8001)

You can customize the ports using:
- `--web-port <PORT>` for the web server
- `--signaling-port <PORT>` for the signaling server
- `--bootnode-port <PORT>` for the bootnode

Example with custom ports:
```bash
cargo run -- --mode all --web-port 3000 --signaling-port 8001 --bootnode-port 4002
```

## Quick Start

1. Start a bootnode:
```bash
cargo run -- --mode bootnode --bootnode-port 4002
```

2. Start additional nodes (each in a new terminal):
```bash
# Connect to default bootnode (localhost:4002)
cargo run -- --mode node

# Or connect to a specific bootnode
cargo run -- --mode node --bootnode-address "/ip4/127.0.0.1/tcp/4002"
```

3. Access the web interface at http://localhost:3000

## Configuration

### Custom Ports

You can customize the ports using command-line arguments:

```bash
cargo run -- --mode all \
  --web-port 3000 \
  --signaling-port 8001 \
  --bootnode-port 4002
```

### Bootnode Connection

Nodes can connect to a specific bootnode by providing its multiaddress:

```bash
# Start a bootnode on a custom port
cargo run -- --mode bootnode --bootnode-port 5000

# Connect a node to the custom bootnode
cargo run -- --mode node --bootnode-address "/ip4/127.0.0.1/tcp/5000"
```

The bootnode address format follows the libp2p multiaddress specification:
- TCP: `/ip4/<ip>/tcp/<port>`
- WebSocket: `/ip4/<ip>/tcp/<port>/ws`

The node will automatically attempt to connect using both TCP and WebSocket transports.

## Distributed Setup

1. Start infrastructure servers:
   ```bash
   # Terminal 1: Start signaling and web servers
   cargo run -- --mode signaling
   ```

2. Start a bootnode:
   ```bash
   # Terminal 2: Start bootnode for peer discovery
   cargo run -- --mode bootnode
   ```

3. Start regular nodes:
   ```bash
   # Terminal 3+: Start as many nodes as you want
   cargo run -- --mode node
   ```

## Common Scenarios

1. Development Testing:
   ```bash
   # Quick start everything on default ports
   cargo run -- --mode all
   ```

2. Running a Signaling Server:
   ```bash
   # Just the infrastructure (no P2P node)
   cargo run -- --mode signaling
   ```

3. Network Bootstrap:
   ```bash
   # Start a bootnode for the P2P network
   cargo run -- --mode bootnode
   ```

4. Regular Node:
   ```bash
   # Start a regular node with web and signaling servers
   cargo run -- --mode node
   ```
   This starts:
   - A regular P2P node
   - Web client server (http://localhost:3000)
   - Signaling server (ws://localhost:8001)

## Example: Setting Up a Local Test Network

1. Start the bootnode:
   ```bash
   # Terminal 1: Bootnode
   cargo run -- --mode bootnode
   ```

2. Start additional nodes (each in a new terminal):
   ```bash
   # Terminal 2+: Additional nodes
   cargo run -- --mode node
   ```
   Each node will run its own:
   - Web client (on port 3000)
   - Signaling server (on port 8001)
   - P2P node

3. Open http://localhost:3000 in your browser for each node

Now you have:
- WebRTC-enabled web clients
- P2P network with bootnode
- Automatic peer discovery
- Direct peer-to-peer messaging

## Network Information

When starting nodes, the application displays:

1. For bootnodes:
   ```
   Bootnode: /ip4/127.0.0.1/tcp/4002
   Bootnode PeerID: [unique identifier]
   ```

2. For regular nodes:
   ```
   Node PeerID: [unique identifier]
   ```

Peer IDs are persistent across restarts:
- Bootnode keys are stored in `data/bootnode/peer_id.key`
- Regular node keys are stored in `data/node/peer_id.key`
- Delete these files to generate new peer IDs

These peer IDs are important for:
- Identifying specific nodes in the network
- Debugging connection issues
- Manual peer connections
- Verifying successful peer discovery

## Troubleshooting

1. If web client shows "Disconnected":
   - Ensure signaling server is running
   - Check if WebSocket port (default: 8001) is accessible

2. If peers don't connect:
   - Verify bootnode is running
   - Check if TCP port (default: 4002) is accessible
   - Ensure no firewall is blocking connections

3. If WebRTC fails:
   - Check browser console for errors
   - Ensure STUN/TURN servers are accessible
   - Verify both peers can reach the signaling server

## Monitoring and Metrics

The application includes comprehensive monitoring for all components:

### Metrics Server

Access metrics at:
- Prometheus metrics: http://localhost:9091/metrics
- JSON stats: http://localhost:9091/stats

### Network Metrics

Monitor P2P network performance:
- Connected peers count
- Messages sent/received
- Bandwidth usage
- Per-peer statistics
- Connection types (direct/STUN/TURN)
- Network latency

### System Metrics

Track system resources:
- CPU usage
- Memory usage
- Disk usage
- Thread count
- Process uptime

### WebSocket Metrics

Monitor signaling server:
- Active connections
- Total connections
- Messages sent/received
- Connection duration

### Prometheus Integration

Use with Prometheus and Grafana:
```yaml
scrape_configs:
  - job_name: 'hippius-libp2p'
    static_configs:
      - targets: ['localhost:9091']
```

### Example Queries

1. Network health:
   ```promql
   rate(p2p_messages_received[5m])  # Message rate
   p2p_connected_peers              # Current peers
   ```

2. System load:
   ```promql
   system_cpu_usage                 # CPU usage
   system_memory_usage             # Memory usage
   ```

3. WebSocket activity:
   ```promql
   ws_active_connections          # Current WebSocket connections
   rate(ws_messages_sent[5m])     # WebSocket message rate
   ```

### Monitoring Dashboard

For visualization:
1. Install Grafana
2. Add Prometheus data source
3. Import the provided dashboard (docs/grafana-dashboard.json)
4. View real-time metrics

### Bandwidth Usage

Track data transfer:
- Total bytes sent/received
- Per-peer bandwidth
- TURN server relay usage
- WebSocket signaling traffic

## Web Client

The project includes a web-based client that demonstrates WebRTC peer-to-peer connections. The web client features:

- Real-time peer-to-peer messaging
- Automatic peer discovery
- Connection status monitoring
- Manual peer connection option
- Clean and intuitive UI

### Using the Web Client

1. Open the web client in multiple browser windows
2. Each client gets a unique peer ID
3. Click on a peer in the list to connect
4. Or enter a peer ID manually and click "Connect"
5. Once connected, you can send messages between peers
6. Messages are sent directly peer-to-peer using WebRTC data channels

### Features

- **Real-time Messaging**: Send and receive messages instantly
- **Peer Management**: See available peers and their connection status
- **Data Channels**: Uses WebRTC data channels for direct peer-to-peer communication
- **Connection Status**: Monitor connection state in real-time
- **Manual Connection**: Connect to peers by ID if they're not automatically discovered

### Technical Details

The web client is built using:
- Pure JavaScript (no frameworks)
- WebRTC API for peer connections
- WebSocket for signaling
- Modern CSS Grid and Flexbox for layout

### Directory Structure

```
web/
├── index.html     # Main HTML file
├── style.css      # Styles and layout
└── webrtc.js      # WebRTC implementation
```

## WebRTC Signaling Server

The project includes a WebRTC signaling server to facilitate peer-to-peer WebRTC connections. The signaling server handles:

- Peer registration and discovery
- SDP offer/answer exchange
- ICE candidate exchange
- Connection state management

### WebSocket API

The signaling server exposes a WebSocket endpoint at `/signal` that accepts the following message types:

```typescript
// Register as a peer
{
  "type": "Register",
  "payload": {
    "peer_id": string
  }
}

// Send WebRTC offer
{
  "type": "Offer",
  "payload": {
    "from": string,
    "to": string,
    "sdp": string
  }
}

// Send WebRTC answer
{
  "type": "Answer",
  "payload": {
    "from": string,
    "to": string,
    "sdp": string
  }
}

// Exchange ICE candidates
{
  "type": "IceCandidate",
  "payload": {
    "from": string,
    "to": string,
    "candidate": string
  }
}
```

### Example WebRTC Client Connection

```javascript
const ws = new WebSocket('ws://localhost:8001/signal');
const peer_id = 'peer_' + Math.random().toString(36).substr(2, 9);

// Register with the signaling server
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'Register',
    payload: { peer_id }
  }));
};

// Handle incoming signaling messages
ws.onmessage = async (event) => {
  const message = JSON.parse(event.data);
  switch (message.type) {
    case 'Offer':
      // Handle incoming WebRTC offer
      break;
    case 'Answer':
      // Handle incoming WebRTC answer
      break;
    case 'IceCandidate':
      // Handle incoming ICE candidate
      break;
  }
};
```

### Security Considerations

- The signaling server should be deployed with TLS in production
- Implement authentication for peer registration
- Use TURN servers for NAT traversal in restricted networks
- Consider rate limiting for DoS protection

## WebRTC Configuration

### TURN Server Support

The application includes full TURN server support for reliable WebRTC connections:
- Configuration in `config/turn_config.json`
- Bandwidth monitoring and logging
- Automatic fallback: Direct → STUN → TURN
- See [TURN Server Setup Guide](docs/TURN_SERVER_SETUP.md)

### Bandwidth Monitoring

Monitor TURN server usage in real-time:
- Bandwidth usage per connection
- Total data transferred
- Connection types (direct/STUN/TURN)
- Logs stored in `logs/turn_bandwidth.log`

### Configuration Options

1. Default (Development):
   ```json
   {
       "stun": {
           "urls": ["stun:stun.l.google.com:19302"]
       }
   }
   ```

2. Production (with TURN):
   ```json
   {
       "stun": {
           "urls": ["stun:stun.l.google.com:19302"]
       },
       "turn": {
           "urls": ["turn:your-server.com:3478"],
           "username": "your-username",
           "credential": "your-password"
       },
       "monitoring": {
           "enabled": true,
           "interval_ms": 5000
       }
   }
   ```

### Monitoring Dashboard

View connection statistics:
1. Open browser console
2. Monitor real-time bandwidth usage
3. Check connection types
4. View total data transferred

For detailed setup instructions, see [TURN Server Setup Guide](docs/TURN_SERVER_SETUP.md)

## Usage Examples

### Topic Management

1. Create and join a new topic:
   ```
   /create-topic tech-discussions
   ```

2. Join an existing topic:
   ```
   /join-topic tech-discussions
   ```

3. Send a message to a topic:
   ```
   /send tech-discussions "Hello everyone! Anyone interested in Rust and P2P?"
   ```

### Network Discovery

The network automatically discovers peers through:
- MDNS for local network peers
- Bootnode connections for initial network entry
- Gossipsub for message propagation

## Architecture

### Transport Layer
- Uses `OrTransport` to combine TCP and WebSocket transports
- Implements protocol upgrading for security and multiplexing
- Supports both direct and web-compatible connections

### Discovery Layer
- MDNS for automatic local peer discovery
- Explicit peer connections for cross-network connectivity
- Automatic peer list management

### Messaging Layer
- Topic-based publish/subscribe using Gossipsub
- Efficient message broadcasting to topic subscribers
- JSON serialization for structured messages

### Security
- Noise protocol for encrypted communications
- PeerId-based peer identification
- Secure transport upgrades

## Development

### Project Structure
```
hippius-libp2p/
├── src/
│   └── main.rs          # Core network implementation
├── Cargo.toml           # Project dependencies
└── README.md           # Project documentation
```

### Key Components
- `P2pServer`: Main network node implementation
- `ServerBehaviour`: Network behavior configuration
- `Message`: Message type definitions
- Transport configuration and setup

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Future Enhancements

- WebRTC transport support for browser compatibility
- DHT-based peer discovery
- Private messaging capabilities
- File sharing functionality
- Web interface for network interaction

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with [libp2p](https://libp2p.io/)
- Inspired by decentralized network principles
- Thanks to the Rust and P2P communities