# Security Architecture

This document describes the security architecture of our decentralized collaborative editor, explaining how we achieve confidentiality, integrity, and availability in a trustless environment.

## Overview

The system uses a multi-layered security approach:
1. End-to-End Encryption (E2EE)
2. Y.js Access Control Layer (ACL)
3. WebRTC Peer-to-Peer Communication
4. Relay Server Architecture

## Security Properties

### Confidentiality
- **End-to-End Encryption**
  - AES-GCM encryption with PBKDF2 key derivation
  - Room-specific encryption keys
  - Keys never transmitted to server
  - Server stores only encrypted data

- **Zero-Knowledge Server**
  - Server cannot read document contents
  - Server operates as blind relay
  - Even if server is compromised, data remains secure

### Integrity
- **Y.js CRDT Properties**
  - Conflict-free replicated data type
  - Cryptographic verification of updates
  - Immutable update history

- **Access Control Layer**
  - Role-based access control
  - Cryptographically signed updates
  - Verification of update authenticity
  - Prevention of unauthorized modifications

### Availability
- **Hybrid Architecture**
  - Direct P2P communication via WebRTC
  - Fallback to relay server when needed
  - Server persistence for offline scenarios
  - Multiple connection paths

## Component Security

### 1. Relay Server
```rust
struct Room {
    peers: HashMap<String, UnboundedSender<Message>>,
    document_state: Vec<u8>, // Encrypted bytes only
    encrypted: bool,
}
```

**Security Properties:**
- Stores only encrypted document state
- Cannot decrypt content without room key
- Cannot forge valid Y.js updates
- Provides availability without trust

### 2. End-to-End Encryption
```javascript
const encryptionUtils = {
    async encrypt(data, key) {
        const iv = window.crypto.getRandomValues(new Uint8Array(12));
        const encrypted = await window.crypto.subtle.encrypt(
            { name: "AES-GCM", iv: iv },
            key,
            data
        );
        // Combine IV and encrypted data
        return new Uint8Array([...iv, ...new Uint8Array(encrypted)]);
    }
}
```

**Security Properties:**
- AES-GCM for authenticated encryption
- Unique IV for each message
- Key derivation using PBKDF2
- Browser's Web Crypto API for secure operations

### 3. Access Control Layer
```javascript
class YjsACL {
    // Implementation in separate file
    // Provides role-based access control
    // Cryptographically signs updates
}
```

**Security Properties:**
- Role-based permissions
- Cryptographic signatures
- Update verification
- Granular access control

## Trust Model

### Trusted Components
1. Client-side code execution
2. Browser's cryptographic primitives
3. Y.js CRDT implementation
4. WebRTC protocol

### Untrusted Components
1. Relay server
2. Network infrastructure
3. Other clients
4. Storage layer

## Attack Scenarios and Mitigations

### 1. Compromised Relay Server
**Threat:** Server attempts to read or modify document content
**Mitigation:**
- End-to-end encryption prevents reading
- Y.js ACL prevents modification
- Cryptographic signatures ensure integrity

### 2. Man-in-the-Middle Attack
**Threat:** Attacker intercepts communication
**Mitigation:**
- E2EE ensures confidentiality
- WebRTC encryption for P2P
- Signature verification prevents tampering

### 3. Malicious Client
**Threat:** Client attempts unauthorized modifications
**Mitigation:**
- Role-based access control
- Cryptographic signatures
- Update verification

### 4. Replay Attack
**Threat:** Attacker replays old messages
**Mitigation:**
- Y.js CRDT properties
- Unique IVs in encryption
- State vector verification

## Security Recommendations

1. **Key Management**
   - Use strong room keys
   - Rotate keys periodically
   - Secure key distribution channel

2. **Access Control**
   - Implement principle of least privilege
   - Regular permission audits
   - Clear role definitions

3. **Monitoring**
   - Log access patterns
   - Monitor for unusual behavior
   - Regular security audits

## Future Enhancements

1. **Key Rotation**
   - Automatic key rotation
   - Key revocation mechanism
   - Forward secrecy

2. **Enhanced ACL**
   - Fine-grained permissions
   - Temporary access grants
   - Audit logging

3. **Additional Security Layers**
   - Multi-factor authentication
   - Hardware security module support
   - Enhanced cryptographic protocols

## Conclusion

The security architecture provides strong guarantees through multiple layers of protection. The combination of E2EE, ACL, and trustless relay server creates a secure collaborative environment where:
- Server cannot read content
- Server cannot modify content
- Clients can verify integrity
- System remains available
- Unauthorized access is prevented
