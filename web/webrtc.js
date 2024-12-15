class TURNManager {
    constructor() {
        this.config = {
            iceServers: [
                { urls: 'stun:stun.l.google.com:19302' },
                // Add your TURN servers here for production
            ]
        };
    }

    createPeerConnection() {
        return new RTCPeerConnection(this.config);
    }
}

class P2PChat {
    constructor() {
        this.peer_id = 'peer_' + Math.random().toString(36).substr(2, 9);
        this.peers = new Map(); // peer_id -> RTCPeerConnection
        this.dataChannels = new Map(); // peer_id -> RTCDataChannel
        this.ws = null;
        this.turnManager = new TURNManager();
        
        this.initializeUI();
        this.connectToSignalingServer();
    }

    initializeUI() {
        this.statusElement = document.getElementById('status');
        this.peerIdElement = document.getElementById('peer-id');
        this.peersListElement = document.getElementById('peers-list');
        this.messagesElement = document.getElementById('messages');
        this.messageInput = document.getElementById('message-input');
        this.sendButton = document.getElementById('send-message');
        this.manualPeerIdInput = document.getElementById('manual-peer-id');
        this.connectManualButton = document.getElementById('connect-manual');

        this.peerIdElement.textContent = `Your ID: ${this.peer_id}`;
        
        this.sendButton.addEventListener('click', () => this.sendMessage());
        this.messageInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') this.sendMessage();
        });
        
        this.connectManualButton.addEventListener('click', () => {
            const peerId = this.manualPeerIdInput.value.trim();
            if (peerId) this.connectToPeer(peerId);
        });
    }

    connectToSignalingServer() {
        this.ws = new WebSocket('ws://localhost:8000/signal');

        this.ws.onopen = () => {
            this.statusElement.textContent = 'Connected to signaling server';
            this.statusElement.classList.add('connected');
            this.ws.send(JSON.stringify({
                type: 'Register',
                payload: { peer_id: this.peer_id }
            }));
        };

        this.ws.onclose = () => {
            this.statusElement.textContent = 'Disconnected from signaling server';
            this.statusElement.classList.remove('connected');
        };

        this.ws.onmessage = async (event) => {
            const message = JSON.parse(event.data);
            await this.handleSignalingMessage(message);
        };
    }

    async handleSignalingMessage(message) {
        switch (message.type) {
            case 'Offer':
                await this.handleOffer(message.payload);
                break;
            case 'Answer':
                await this.handleAnswer(message.payload);
                break;
            case 'IceCandidate':
                await this.handleIceCandidate(message.payload);
                break;
        }
    }

    async connectToPeer(peerId) {
        if (this.peers.has(peerId)) return;

        const peerConnection = await this.createPeerConnection(peerId);
        this.peers.set(peerId, peerConnection);

        // Create data channel
        const dataChannel = peerConnection.createDataChannel('chat');
        this.setupDataChannel(dataChannel, peerId);

        // Create and send offer
        const offer = await peerConnection.createOffer();
        await peerConnection.setLocalDescription(offer);
        
        this.ws.send(JSON.stringify({
            type: 'Offer',
            payload: {
                from: this.peer_id,
                to: peerId,
                sdp: peerConnection.localDescription.sdp
            }
        }));
    }

    async createPeerConnection(peerId) {
        const pc = this.turnManager.createPeerConnection();
        this.setupPeerConnection(pc, peerId);
        return pc;
    }

    async handleOffer({ from, sdp }) {
        const peerConnection = await this.createPeerConnection(from);
        this.peers.set(from, peerConnection);

        // Handle data channel
        peerConnection.ondatachannel = (event) => {
            this.setupDataChannel(event.channel, from);
        };

        // Set remote description and create answer
        await peerConnection.setRemoteDescription(new RTCSessionDescription({ type: 'offer', sdp }));
        const answer = await peerConnection.createAnswer();
        await peerConnection.setLocalDescription(answer);

        this.ws.send(JSON.stringify({
            type: 'Answer',
            payload: {
                from: this.peer_id,
                to: from,
                sdp: peerConnection.localDescription.sdp
            }
        }));
    }

    async handleAnswer({ from, sdp }) {
        const peerConnection = this.peers.get(from);
        if (peerConnection) {
            await peerConnection.setRemoteDescription(new RTCSessionDescription({ type: 'answer', sdp }));
        }
    }

    async handleIceCandidate({ from, candidate }) {
        const peerConnection = this.peers.get(from);
        if (peerConnection) {
            await peerConnection.addIceCandidate(new RTCIceCandidate(JSON.parse(candidate)));
        }
    }

    setupPeerConnection(peerConnection, peerId) {
        peerConnection.onicecandidate = (event) => {
            if (event.candidate) {
                this.ws.send(JSON.stringify({
                    type: 'IceCandidate',
                    payload: {
                        from: this.peer_id,
                        to: peerId,
                        candidate: JSON.stringify(event.candidate)
                    }
                }));
            }
        };

        peerConnection.onconnectionstatechange = () => {
            this.updatePeerStatus(peerId, peerConnection.connectionState);
        };
    }

    setupDataChannel(dataChannel, peerId) {
        this.dataChannels.set(peerId, dataChannel);

        dataChannel.onopen = () => {
            this.updatePeerStatus(peerId, 'connected');
            this.addMessage('System', `Connected to peer: ${peerId}`);
        };

        dataChannel.onclose = () => {
            this.updatePeerStatus(peerId, 'disconnected');
            this.addMessage('System', `Disconnected from peer: ${peerId}`);
        };

        dataChannel.onmessage = (event) => {
            this.addMessage(peerId, event.data);
        };
    }

    updatePeerStatus(peerId, status) {
        let peerElement = document.querySelector(`[data-peer-id="${peerId}"]`);
        if (!peerElement) {
            peerElement = document.createElement('div');
            peerElement.className = 'peer-item';
            peerElement.setAttribute('data-peer-id', peerId);
            this.peersListElement.appendChild(peerElement);
        }
        peerElement.textContent = `${peerId} (${status})`;
    }

    addMessage(from, content) {
        const messageElement = document.createElement('div');
        messageElement.className = `message ${from === this.peer_id ? 'sent' : 'received'}`;
        messageElement.textContent = `${from}: ${content}`;
        this.messagesElement.appendChild(messageElement);
        this.messagesElement.scrollTop = this.messagesElement.scrollHeight;
    }

    sendMessage() {
        const content = this.messageInput.value.trim();
        if (!content) return;

        this.dataChannels.forEach((channel) => {
            if (channel.readyState === 'open') {
                channel.send(content);
            }
        });

        this.addMessage(this.peer_id, content);
        this.messageInput.value = '';
    }
}

// Initialize the P2P chat application
window.addEventListener('load', () => {
    new P2PChat();
});
