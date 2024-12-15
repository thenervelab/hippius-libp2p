class TURNMonitor {
    constructor(peerConnection, config) {
        this.pc = peerConnection;
        this.config = config;
        this.stats = {
            bytesReceived: 0,
            bytesSent: 0,
            timestamp: Date.now()
        };
        this.logFile = config.monitoring.log_file;
    }

    async startMonitoring() {
        if (!this.config.monitoring.enabled) return;
        
        setInterval(async () => {
            const stats = await this.pc.getStats();
            let currentStats = {
                bytesReceived: 0,
                bytesSent: 0,
                timestamp: Date.now()
            };

            stats.forEach(report => {
                if (report.type === 'transport') {
                    currentStats.bytesReceived += report.bytesReceived || 0;
                    currentStats.bytesSent += report.bytesSent || 0;
                }
            });

            // Calculate bandwidth
            const duration = (currentStats.timestamp - this.stats.timestamp) / 1000;
            const receivedBandwidth = (currentStats.bytesReceived - this.stats.bytesReceived) / duration;
            const sentBandwidth = (currentStats.bytesSent - this.stats.bytesSent) / duration;

            // Log bandwidth usage
            const logEntry = {
                timestamp: new Date().toISOString(),
                receivedBandwidth: `${(receivedBandwidth / 1024).toFixed(2)} KB/s`,
                sentBandwidth: `${(sentBandwidth / 1024).toFixed(2)} KB/s`,
                totalReceived: `${(currentStats.bytesReceived / (1024 * 1024)).toFixed(2)} MB`,
                totalSent: `${(currentStats.bytesSent / (1024 * 1024)).toFixed(2)} MB`
            };

            console.log('TURN Bandwidth Usage:', logEntry);
            this.logToFile(logEntry);

            this.stats = currentStats;
        }, this.config.monitoring.interval_ms);
    }

    logToFile(entry) {
        // In a real implementation, you'd want to send this to your server
        // for logging. Here we just use console.log as an example.
        const logEntry = `${entry.timestamp} - RX: ${entry.receivedBandwidth} - TX: ${entry.sentBandwidth}`;
        console.log(logEntry);
    }
}

class TURNManager {
    constructor() {
        this.loadConfig();
    }

    async loadConfig() {
        try {
            const response = await fetch('/config/turn_config.json');
            this.config = await response.json();
        } catch (error) {
            console.error('Failed to load TURN config:', error);
            // Fallback to default configuration
            this.config = {
                stun: {
                    urls: ['stun:stun.l.google.com:19302']
                }
            };
        }
    }

    getIceServers() {
        const iceServers = [];

        // Add STUN servers
        if (this.config.stun) {
            iceServers.push({
                urls: this.config.stun.urls
            });
        }

        // Add TURN servers if configured
        if (this.config.turn) {
            iceServers.push({
                urls: this.config.turn.urls,
                username: this.config.turn.username,
                credential: this.config.turn.credential
            });
        }

        return iceServers;
    }

    createPeerConnection() {
        const config = {
            iceServers: this.getIceServers(),
            iceTransportPolicy: 'all', // Use 'relay' to force TURN
            bundlePolicy: 'max-bundle',
            rtcpMuxPolicy: 'require',
            enableDtlsSrtp: true
        };

        const pc = new RTCPeerConnection(config);
        
        // Create and start bandwidth monitor
        if (this.config.monitoring?.enabled) {
            const monitor = new TURNMonitor(pc, this.config);
            monitor.startMonitoring();
        }

        return pc;
    }
}

// Export for use in webrtc.js
window.TURNManager = TURNManager;
