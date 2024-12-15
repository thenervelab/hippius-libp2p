# Y.js Collaborative Editor Integration Guide

This guide explains how to integrate our collaborative editing system into your web application. Our system provides real-time collaboration with end-to-end encryption support.

## Features

- 🔄 Real-time collaboration using Y.js
- 🔐 End-to-end encryption (optional)
- 🏠 Multiple room support
- 👥 User presence awareness
- 🎨 User cursors and colors
- 📝 Text editor integration (CodeMirror)

## Quick Start

### 1. Include Required Dependencies

```html
<script type="module">
    import * as Y from 'https://cdn.jsdelivr.net/npm/yjs@13.6.8/dist/yjs.mjs';
    import { CodemirrorBinding } from 'https://cdn.jsdelivr.net/npm/y-codemirror@3.0.1/dist/y-codemirror.mjs';
    window.Y = Y;
    window.CodemirrorBinding = CodemirrorBinding;
</script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/codemirror.min.js"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/codemirror.min.css">
```

### 2. Basic Setup

```javascript
// Initialize Y.js document
const ydoc = new Y.Doc();

// Create custom provider
const provider = new CustomSignalingProvider(ydoc, roomId, encryptionKey);

// Initialize CodeMirror
const editor = CodeMirror(document.getElementById('editor'), {
    mode: 'text/plain',
    lineNumbers: true
});

// Bind Y.js to CodeMirror
const binding = new CodemirrorBinding(
    ydoc.getText('codemirror'),
    editor,
    provider.awareness
);
```

## Encryption Support

### Setting Up Encryption

```javascript
// When creating/joining a room
const encryptionKey = "your-secret-key";
const provider = new CustomSignalingProvider(ydoc, roomId, encryptionKey);
```

### Encryption Utilities

```javascript
const encryptionUtils = {
    // Generate encryption key from password
    async generateKey(password) {
        const enc = new TextEncoder();
        const keyMaterial = await window.crypto.subtle.importKey(
            "raw",
            enc.encode(password),
            { name: "PBKDF2" },
            false,
            ["deriveBits", "deriveKey"]
        );

        return window.crypto.subtle.deriveKey(
            {
                name: "PBKDF2",
                salt: enc.encode("y-js-collab"),
                iterations: 100000,
                hash: "SHA-256"
            },
            keyMaterial,
            { name: "AES-GCM", length: 256 },
            true,
            ["encrypt", "decrypt"]
        );
    },

    // Encrypt data
    async encrypt(data, key) {
        const iv = window.crypto.getRandomValues(new Uint8Array(12));
        const enc = new TextEncoder();
        
        const encrypted = await window.crypto.subtle.encrypt(
            {
                name: "AES-GCM",
                iv: iv
            },
            key,
            typeof data === 'string' ? enc.encode(data) : data
        );

        const encryptedContent = new Uint8Array(encrypted);
        const result = new Uint8Array(iv.length + encryptedContent.length);
        result.set(iv);
        result.set(encryptedContent, iv.length);
        
        return result;
    },

    // Decrypt data
    async decrypt(data, key) {
        const iv = data.slice(0, 12);
        const content = data.slice(12);
        
        return await window.crypto.subtle.decrypt(
            {
                name: "AES-GCM",
                iv: iv
            },
            key,
            content
        );
    }
};
```

## Custom Signaling Provider

Here's a complete implementation of the custom provider that handles both regular and encrypted communication:

```javascript
class CustomSignalingProvider {
    constructor(ydoc, roomId, encryptionKey = null) {
        this.ydoc = ydoc;
        this.roomId = roomId;
        this.encryptionKey = encryptionKey;
        this.ws = null;
        this.peers = new Set();
        this.awareness = new Y.Awareness(ydoc);
        this.connect();

        // Set up local user state
        this.awareness.setLocalState({
            user: {
                name: 'User ' + ydoc.clientID,
                color: this.getRandomColor()
            }
        });
    }

    getRandomColor() {
        return '#' + Math.floor(Math.random()*16777215).toString(16);
    }

    async connect() {
        this.ws = new WebSocket('ws://127.0.0.1:8081');
        
        this.ws.onopen = async () => {
            let joinPayload = {
                user_id: this.ydoc.clientID,
                room_id: this.roomId,
                encrypted_data: null
            };

            // Test encryption if key provided
            if (this.encryptionKey) {
                const key = await encryptionUtils.generateKey(this.encryptionKey);
                const encrypted = await encryptionUtils.encrypt('test', key);
                joinPayload.encrypted_data = btoa(String.fromCharCode.apply(null, encrypted));
            }

            this.ws.send(JSON.stringify({ 
                type: 'Join',
                payload: joinPayload
            }));
        };

        this.ws.onmessage = async (event) => {
            const message = JSON.parse(event.data);
            
            if (message.type === 'SyncUpdate' && this.encryptionKey) {
                const key = await encryptionUtils.generateKey(this.encryptionKey);
                const decrypted = await encryptionUtils.decrypt(
                    new Uint8Array(atob(message.payload.update).split('').map(c => c.charCodeAt(0))),
                    key
                );
                Y.applyUpdate(this.ydoc, new Uint8Array(decrypted));
            } else if (message.type === 'SyncUpdate') {
                Y.applyUpdate(this.ydoc, new Uint8Array(atob(message.payload.update).split('').map(c => c.charCodeAt(0))));
            }
        };

        this.ws.onclose = () => {
            setTimeout(() => this.connect(), 5000);
        };
    }

    async send(message) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            if (this.encryptionKey && message.type === 'SyncUpdate') {
                const key = await encryptionUtils.generateKey(this.encryptionKey);
                const encrypted = await encryptionUtils.encrypt(new Uint8Array(message.payload.update), key);
                message.payload.update = btoa(String.fromCharCode.apply(null, encrypted));
            }
            this.ws.send(JSON.stringify(message));
        }
    }

    disconnect() {
        if (this.ws) {
            this.ws.close();
        }
    }
}
```

## Room Management

### Creating a Room

```javascript
function createRoom(encryptionKey = null) {
    const roomId = generateUUID(); // Use your preferred UUID generation
    const ydoc = new Y.Doc();
    const provider = new CustomSignalingProvider(ydoc, roomId, encryptionKey);
    
    return { roomId, ydoc, provider };
}
```

### Joining a Room

```javascript
function joinRoom(roomId, encryptionKey = null) {
    const ydoc = new Y.Doc();
    const provider = new CustomSignalingProvider(ydoc, roomId, encryptionKey);
    
    return { ydoc, provider };
}
```

## User Presence

### Updating User State

```javascript
provider.awareness.setLocalState({
    user: {
        name: 'User Name',
        color: '#ff0000',
        cursor: null
    }
});
```

### Listening for User Changes

```javascript
provider.awareness.on('change', ({ added, updated, removed }) => {
    const states = provider.awareness.getStates();
    // Update UI with user states
});
```

## Best Practices

1. **Error Handling**
   - Always handle WebSocket connection errors
   - Implement reconnection logic
   - Validate encryption keys

2. **Security**
   - Never store encryption keys in localStorage
   - Use secure password generation for encryption keys
   - Implement proper key exchange mechanisms

3. **Performance**
   - Implement debouncing for cursor updates
   - Use efficient encoding for large documents
   - Clean up resources when disconnecting

4. **UX Considerations**
   - Show connection status
   - Indicate encrypted rooms
   - Display user presence
   - Show typing indicators

## Example Implementation

See our complete example in `yjs-example-encrypted.html` for a full implementation including:
- Room creation and joining
- End-to-end encryption
- User presence
- Error handling
- UI integration

## Troubleshooting

### Common Issues

1. **Connection Issues**
   - Check WebSocket server URL
   - Verify network connectivity
   - Check for firewall blocking

2. **Encryption Problems**
   - Verify encryption key format
   - Check for encoding issues
   - Validate key exchange

3. **Sync Issues**
   - Verify Y.js document structure
   - Check update application
   - Validate awareness states

## Support

For issues and feature requests, please:
1. Check the example implementation
2. Review troubleshooting guide
3. Open an issue on GitHub

## License

This project is licensed under the MIT License.
