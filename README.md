# Hippius LibP2P Network

A robust peer-to-peer networking application built with Rust and LibP2P, supporting multiple transport protocols and distributed peer discovery. This project demonstrates how to build a flexible P2P network that can work across different network environments.

## Features

- **Multi-Transport Support**
  - TCP for direct connections
  - WebSocket for web-compatible connections
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

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo (comes with Rust)
- Network connectivity (for peer discovery)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/hippius-libp2p.git
   cd hippius-libp2p
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

### Running

1. Start a bootnode (first node in the network):
   ```bash
   cargo run --release -- --bootnode
   ```

2. Start regular nodes (in separate terminals):
   ```bash
   cargo run --release
   ```

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