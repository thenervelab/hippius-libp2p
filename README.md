# Decentralized Y.js Collaborative Editor with libp2p

A decentralized collaborative text editor built with Rust, Y.js, and libp2p, supporting multiple rooms and true peer-to-peer collaboration with encrypted communication.

## Features

- 📝 Real-time collaborative text editing
- 🏠 Multiple room support
- 🌐 Decentralized peer-to-peer architecture using libp2p
- 👥 User presence awareness
- 🔄 Automatic reconnection
- 💾 Per-room document state persistence
- 🔒 End-to-end encryption support
- 🌍 DNS and TCP transport support
- 🔐 Persistent peer identities
- 📡 Bootnode discovery

## Prerequisites

- Rust (latest stable version)
- Cargo
- Python 3 (for running the local web server)

## Dependencies

```toml
[dependencies]
tokio = { version = "1.35", features = ["full"] }
libp2p = { version = "0.52", features = [
    "identify",
    "noise",
    "tcp",
    "dns",
    "yamux",
    "mdns",
    "gossipsub"
] }
tokio-tungstenite = "0.20"
futures-util = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.6", features = ["v4"] }
tracing = "0.1"
tracing-subscriber = "0.3"
base64 = "0.21"
clap = { version = "4.4", features = ["derive"] }
```

## Quick Start

1. Clone the repository:
```bash
git clone <your-repo-url>
cd hippius-libp2p
```

2. Build the project:
```bash
cargo build --release
```

3. Start a bootnode:
```bash
cargo run -- --bootnode
# Note the peer ID that is displayed
```

4. Start regular nodes connecting to the bootnode:
```bash
# Replace PEER_ID with the bootnode's peer ID from step 3
cargo run -- --bootnode-addr "/dns4/localhost/tcp/58455/p2p/PEER_ID"
```

5. Start the web server:
```bash
python3 -m http.server 8000
```

6. Open one of the following applications:
- Basic editor: http://localhost:8000/yjs-example.html
- Multi-room editor: http://localhost:8000/yjs-example-rooms.html
- Encrypted editor: http://localhost:8000/yjs-example-encrypted.html

## libp2p Architecture

### Network Components

1. **Bootnode**
   - Acts as a rendezvous point for other nodes
   - Runs with persistent identity (stored in `bootnode_key.json`)
   - Listens on all interfaces (0.0.0.0)
   - Default port: 58455

2. **Regular Nodes**
   - Connect to bootnode for peer discovery
   - Maintain persistent identity (stored in `node_key.json`)
   - Use local ports for listening
   - Support both IP and DNS addressing

3. **Transport Layer**
   - TCP transport with noise encryption
   - DNS resolution support
   - Yamux multiplexing
   - mDNS for local peer discovery
   - Gossipsub for message broadcasting

### Security Features

1. **Persistent Identities**
   - Each node maintains a persistent Ed25519 keypair
   - Bootnode identity stored in `bootnode_key.json`
   - Regular node identity stored in `node_key.json`
   - Used for message signing and encryption

2. **Transport Security**
   - Noise protocol for encrypted communication
   - Authentication of peer identities
   - Perfect forward secrecy
   - Man-in-the-middle protection

3. **End-to-End Encryption**
   - Room-specific encryption keys
   - AES-GCM encryption
   - PBKDF2 key derivation
   - Server never sees decrypted content

### Message Flow

1. **Node Discovery**
   ```
   Regular Node → Bootnode → Other Nodes
        │            │            │
        └────────────┴────────────┘
              Direct P2P
   ```

2. **Room Messages**
   ```
   Node A → Gossipsub → All Room Peers
   ```

### Using DNS Addresses

The system supports both IP and DNS addressing:

1. IP Address Format:
```bash
/ip4/127.0.0.1/tcp/58455/p2p/PEER_ID
```

2. DNS Address Format:
```bash
/dns4/localhost/tcp/58455/p2p/PEER_ID
```

### Running Behind NAT

When running nodes behind NAT:

1. Configure port forwarding for the bootnode:
   - Forward TCP port 58455 to the bootnode
   - Use public IP or domain name in bootnode address

2. Regular nodes can use DNS:
```bash
cargo run -- --bootnode-addr "/dns4/your-domain.com/tcp/58455/p2p/PEER_ID"
```

## Development

### Project Structure
```
hippius-libp2p/
├── src/
│   └── main.rs          # Rust p2p node implementation
├── bootnode_key.json    # Persistent bootnode identity
├── node_key.json        # Persistent regular node identity
├── yjs-example.html     # Basic Y.js client
├── yjs-example-rooms.html # Multi-room Y.js client
├── yjs-example-encrypted.html # Encrypted Y.js client
├── Cargo.toml           # Rust dependencies
└── README.md           # This file
```

### Adding New Features

1. Server-side (main.rs):
- Extend `NetworkBehaviour` implementation
- Add new message types to Gossipsub
- Implement new libp2p protocols as needed

2. Client-side:
- Add UI elements for new features
- Implement message handling
- Update room management code

## Future Enhancements

Planned features:
1. QUIC transport support
2. DHT-based peer discovery
3. NAT traversal improvements
4. WebRTC transport
5. Bandwidth optimization
6. Enhanced security features

## Contributing

1. Fork the repository
2. Create your feature branch
3. Commit your changes
4. Push to the branch
5. Create a new Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.
