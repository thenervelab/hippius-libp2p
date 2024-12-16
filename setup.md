# Server Setup Guide

This guide provides step-by-step instructions for setting up a Hippius LibP2P node with IPFS integration on a Linux server.

## System Requirements

- Ubuntu 22.04 LTS or newer
- 2 CPU cores
- 4GB RAM minimum
- 20GB SSD storage
- Public IPv4 address

## Initial Server Setup

### 1. Update System

```bash
sudo apt update && sudo apt upgrade -y
```

### 2. Configure Time Synchronization

```bash
# Install chrony
sudo apt install -y chrony

# Backup original config
sudo cp /etc/chrony/chrony.conf /etc/chrony/chrony.conf.bak

# Configure chrony
sudo tee /etc/chrony/chrony.conf > /dev/null << 'EOF'
pool pool.ntp.org iburst
initstepslew 10 pool.ntp.org
driftfile /var/lib/chrony/drift
local stratum 10
makestep 1.0 3
rtcsync
EOF

# Restart chrony
sudo systemctl restart chrony
sudo systemctl enable chrony

# Verify
chronyc tracking
```

### 3. Configure Firewall

```bash
# Install UFW if not present
sudo apt install -y ufw

# Set default policies
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Allow SSH (modify port if needed)
sudo ufw allow 22/tcp

# Allow Hippius LibP2P ports
sudo ufw allow 3000/tcp  # Web interface
sudo ufw allow 8001/tcp  # Signaling server
sudo ufw allow 4002/tcp  # P2P bootnode
sudo ufw allow 9091/tcp  # Metrics

# Allow IPFS ports
sudo ufw allow 4001/tcp  # IPFS swarm
sudo ufw allow 5001/tcp  # IPFS API
sudo ufw allow 8080/tcp  # IPFS Gateway

# Enable firewall
sudo ufw enable

# Verify status
sudo ufw status numbered
```

## Install Required Software

### 1. Install Caddy Web Server

```bash
# Install Caddy
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update
sudo apt install caddy
```

### 2. Install IPFS (Kubo)

```bash
# Download latest Kubo
wget https://dist.ipfs.tech/kubo/latest/kubo_latest_linux-amd64.tar.gz

# Extract
tar -xvzf kubo_latest_linux-amd64.tar.gz

# Install
cd kubo
sudo bash install.sh

# Initialize IPFS
ipfs init

# Configure IPFS
ipfs config Addresses.API /ip4/0.0.0.0/tcp/5001
ipfs config Addresses.Gateway /ip4/0.0.0.0/tcp/8080
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Origin '["*"]'
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Methods '["PUT", "POST", "GET"]'
```

### 3. Install Hippius LibP2P

```bash
# Create directory
sudo mkdir -p /opt/hippius
cd /opt/hippius

# Download binary (replace VERSION with actual version)
sudo wget https://github.com/thenervelab/hippius-libp2p/releases/download/vVERSION/hippius-libp2p

# Make executable
sudo chmod +x hippius-libp2p

# Create data directory
sudo mkdir -p /var/lib/hippius
```

## Service Configuration

### 1. IPFS Service

```bash
# Create systemd service file
sudo tee /etc/systemd/system/ipfs.service > /dev/null << 'EOF'
[Unit]
Description=IPFS Daemon
After=network.target

[Service]
Type=simple
Environment=IPFS_PATH=/var/lib/ipfs
ExecStart=/usr/local/bin/ipfs daemon
Restart=always
RestartSec=10
User=ipfs
Group=ipfs

[Install]
WantedBy=multi-user.target
EOF

# Create IPFS user
sudo useradd -r -s /bin/false ipfs

# Set up directories
sudo mkdir -p /var/lib/ipfs
sudo chown -R ipfs:ipfs /var/lib/ipfs

# Start IPFS service
sudo systemctl daemon-reload
sudo systemctl enable ipfs
sudo systemctl start ipfs
```

### 2. Hippius LibP2P Service

```bash
# Create systemd service file
sudo tee /etc/systemd/system/hippius.service > /dev/null << 'EOF'
[Unit]
Description=Hippius LibP2P Node
After=network.target

[Service]
Type=simple
ExecStart=/opt/hippius/hippius-libp2p --mode all \
    --web-port 3000 \
    --signaling-port 8001 \
    --bootnode-port 4002
Restart=always
RestartSec=10
User=hippius
Group=hippius
WorkingDirectory=/var/lib/hippius

[Install]
WantedBy=multi-user.target
EOF

# Create Hippius user
sudo useradd -r -s /bin/false hippius

# Set up directories
sudo chown -R hippius:hippius /var/lib/hippius
sudo chown -R hippius:hippius /opt/hippius

# Start Hippius service
sudo systemctl daemon-reload
sudo systemctl enable hippius
sudo systemctl start hippius
```

## Reverse Proxy Setup

You can choose between two options for reverse proxy setup:

### Option 1: Caddy (Simple, Self-hosted)

```bash
# Create Caddy configuration
sudo tee /etc/caddy/Caddyfile > /dev/null << 'EOF'
{
    admin off
    auto_https disable_redirects
}

:80 {
    # Web interface
    handle /app/* {
        reverse_proxy localhost:3000
    }

    # WebSocket signaling
    handle /signal/* {
        reverse_proxy localhost:8001
    }

    # Metrics
    handle /metrics {
        reverse_proxy localhost:9091
        basicauth {
            metrics JDJhJDEwJHBsNEZXWk9ZdnQuOWZwYnlVcUx1TE9ZYk5Gd2FmSzBxY0ZQYlFJUTVkSzBRWnVpQXNpVTJL
        }
    }

    # IPFS Gateway
    handle /ipfs/* {
        reverse_proxy localhost:8080
    }

    # IPFS API
    handle /api/v0/* {
        reverse_proxy localhost:5001
    }

    # Root redirect
    handle {
        redir /app{uri}
    }

    # Security headers
    header {
        # enable HSTS
        Strict-Transport-Security max-age=31536000;
        # disable clients from sniffing the media type
        X-Content-Type-Options nosniff
        # keep referrer data off of HTTP connections
        Referrer-Policy no-referrer-when-downgrade
        # Enable cross-site filter (XSS) and tell browser to block detected attacks
        X-XSS-Protection "1; mode=block"
        # Prevent site from being embedded in iframes
        X-Frame-Options DENY
    }
}

# HTTPS version (uncomment and modify for production)
yourdomain.com {
    # Web interface
    handle /app/* {
        reverse_proxy localhost:3000
    }

    # WebSocket signaling
    handle /signal/* {
        reverse_proxy localhost:8001
    }

    # Metrics
    handle /metrics {
        reverse_proxy localhost:9091
        basicauth {
            metrics JDJhJDEwJHBsNEZXWk9ZdnQuOWZwYnlVcUx1TE9ZYk5Gd2FmSzBxY0ZQYlFJUTVkSzBRWnVpQXNpVTJL
        }
    }

    # IPFS Gateway
    handle /ipfs/* {
        reverse_proxy localhost:8080
    }

    # IPFS API
    handle /api/v0/* {
        reverse_proxy localhost:5001
    }



    # Security headers
    header {
        Strict-Transport-Security max-age=31536000;
        X-Content-Type-Options nosniff
        Referrer-Policy no-referrer-when-downgrade
        X-XSS-Protection "1; mode=block"
        X-Frame-Options DENY
    }
}
EOF

# Restart Caddy
sudo systemctl restart caddy
```

### Option 2: Cloudflare Tunnel (Enterprise-grade, Zero Trust)

Cloudflare Tunnel provides enterprise-level security and DDoS protection without exposing ports to the internet.

#### 1. Install cloudflared

```bash
# Download and install cloudflared
curl -L --output cloudflared.deb https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
sudo dpkg -i cloudflared.deb

# Create cloudflared user
sudo useradd -r -s /bin/false cloudflared
```

#### 2. Set up Cloudflare Tunnel

```bash
# Login to Cloudflare (this will open a browser)
cloudflared tunnel login

# Create a tunnel
cloudflared tunnel create hippius-tunnel

# Note your tunnel ID from the output
TUNNEL_ID=<your-tunnel-id>

# Create config directory
sudo mkdir -p /etc/cloudflared
```

#### 3. Configure Tunnel

```bash
# Create configuration file
sudo tee /etc/cloudflared/config.yml << EOF
tunnel: ${TUNNEL_ID}
credentials-file: /etc/cloudflared/${TUNNEL_ID}.json

ingress:
  # Web Interface
  - hostname: hippius.yourdomain.com
    service: http://localhost:3000
    originRequest:
      noTLSVerify: true

  # WebSocket Signaling
  - hostname: ws-hippius.yourdomain.com
    service: http://localhost:8001
    originRequest:
      noTLSVerify: true
      connectTimeout: 10s
    # Enable WebSocket
    originRequest:
      websocket: true

  # IPFS Gateway
  - hostname: ipfs-hippius.yourdomain.com
    service: http://localhost:8080
    originRequest:
      noTLSVerify: true
      connectTimeout: 30s

  # IPFS API
  - hostname: ipfs-api-hippius.yourdomain.com
    service: http://localhost:5001
    originRequest:
      noTLSVerify: true
      connectTimeout: 30s

  # Metrics (with authentication)
  - hostname: metrics-hippius.yourdomain.com
    service: http://localhost:9091
    originRequest:
      noTLSVerify: true
    
  # Catch-all rule
  - service: http_status:404
EOF

# Copy tunnel credentials
sudo cp ~/.cloudflared/${TUNNEL_ID}.json /etc/cloudflared/

# Set permissions
sudo chown -R cloudflared:cloudflared /etc/cloudflared
sudo chmod 600 /etc/cloudflared/config.yml
sudo chmod 600 /etc/cloudflared/${TUNNEL_ID}.json
```

#### 4. Create Systemd Service

```bash
sudo tee /etc/systemd/system/cloudflared.service << EOF
[Unit]
Description=Cloudflare Tunnel
After=network.target

[Service]
Type=simple
User=cloudflared
ExecStart=/usr/bin/cloudflared tunnel --config /etc/cloudflared/config.yml run
Restart=always
RestartSec=5
TimeoutStartSec=0
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

# Start and enable service
sudo systemctl daemon-reload
sudo systemctl enable cloudflared
sudo systemctl start cloudflared
```

#### 5. Configure DNS

```bash
# Route traffic through the tunnel
cloudflared tunnel route dns ${TUNNEL_ID} hippius-yourdomain-com
cloudflared tunnel route dns ${TUNNEL_ID} ws-hippius-yourdomain-com
cloudflared tunnel route dns ${TUNNEL_ID} metrics-hippius-yourdomain-com
cloudflared tunnel route dns ${TUNNEL_ID} ipfs-hippius-yourdomain-com
cloudflared tunnel route dns ${TUNNEL_ID} ipfs-api-hippius-yourdomain-com
```

#### 6. Update Firewall Rules

With Cloudflare Tunnel, you don't need to expose ports to the internet:

```bash
# Reset UFW
sudo ufw reset

# Set default policies
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Allow SSH
sudo ufw allow 22/tcp

# Allow local access only
sudo ufw allow from 127.0.0.1 to any port 3000
sudo ufw allow from 127.0.0.1 to any port 8001
sudo ufw allow from 127.0.0.1 to any port 9091
sudo ufw allow from 127.0.0.1 to any port 4002
sudo ufw allow from 127.0.0.1 to any port 8080
sudo ufw allow from 127.0.0.1 to any port 5001

# Enable firewall
sudo ufw enable
```

#### 7. Configure Cloudflare Security Settings

In the Cloudflare Dashboard:

1. **SSL/TLS**
   - Set SSL/TLS encryption mode to "Full (strict)"
   - Enable TLS 1.3
   - Enable Automatic HTTPS Rewrites

2. **Security**
   - Enable Bot Fight Mode
   - Enable Browser Integrity Check
   - Configure WAF rules
   - Set up rate limiting rules

3. **Network**
   - Enable WebSockets
   - Enable HTTP/3
   - Enable Argo Smart Routing (optional)

4. **Zero Trust (optional)**
   - Configure Access policies
   - Set up identity provider
   - Create application policies

#### 8. Monitoring and Logs

```bash
# View tunnel status
cloudflared tunnel info ${TUNNEL_ID}

# Check tunnel logs
sudo journalctl -u cloudflared -f

# View metrics
curl -s http://localhost:9091/metrics
```

#### Benefits of Cloudflare Tunnel

1. **Security**
   - Enterprise-grade DDoS protection
   - WAF (Web Application Firewall)
   - Bot protection
   - Zero Trust security model
   - No exposed ports

2. **Performance**
   - Global CDN
   - Automatic SSL/TLS
   - HTTP/3 support
   - Argo Smart Routing

3. **Monitoring**
   - Real-time analytics
   - Attack monitoring
   - Performance metrics
   - Detailed logs

4. **Management**
   - Zero-config SSL
   - Automatic updates
   - Centralized management
   - Access control

Choose Option 1 (Caddy) for simple, self-hosted setups or Option 2 (Cloudflare Tunnel) for enterprise-grade security and features.

## Verification

### 1. Check Service Status

```bash
# Check IPFS
sudo systemctl status ipfs

# Check Hippius
sudo systemctl status hippius

# Check Caddy or Cloudflare Tunnel
sudo systemctl status caddy
sudo systemctl status cloudflared
```

### 2. Check Logs

```bash
# IPFS logs
sudo journalctl -u ipfs -f

# Hippius logs
sudo journalctl -u hippius -f

# Caddy logs
sudo journalctl -u caddy -f

# Cloudflare Tunnel logs
sudo journalctl -u cloudflared -f
```

### 3. Test Endpoints

```bash
# Test web interface
curl http://localhost/app/

# Test IPFS
curl http://localhost/api/v0/version

# Test metrics (replace password)
curl -u metrics:your_password http://localhost/metrics
```

## Maintenance

### Updating Services

```bash
# Update IPFS
wget https://dist.ipfs.tech/kubo/latest/kubo_latest_linux-amd64.tar.gz
tar -xvzf kubo_latest_linux-amd64.tar.gz
cd kubo
sudo bash install.sh
sudo systemctl restart ipfs

# Update Hippius (replace VERSION)
cd /opt/hippius
sudo wget https://github.com/thenervelab/hippius-libp2p/releases/download/vVERSION/hippius-libp2p
sudo chmod +x hippius-libp2p
sudo systemctl restart hippius
```

### Backup

```bash
# Backup IPFS data
sudo tar -czf ipfs-backup.tar.gz /var/lib/ipfs

# Backup Hippius data
sudo tar -czf hippius-backup.tar.gz /var/lib/hippius
```

## Troubleshooting

### Common Issues

1. **Services won't start**
   - Check logs: `sudo journalctl -u [service-name] -n 100`
   - Verify permissions: `ls -la /var/lib/[service-name]`
   - Check ports: `sudo netstat -tulpn | grep LISTEN`

2. **Network connectivity issues**
   - Check firewall: `sudo ufw status`
   - Verify ports: `sudo lsof -i :[port]`
   - Test network: `curl -v telnet://localhost:[port]`

3. **High resource usage**
   - Check system resources: `htop`
   - Monitor disk usage: `df -h`
   - Check service metrics: `http://localhost/metrics`

## Security Considerations

1. **Access Control**
   - Change default passwords in Caddy configuration
   - Use strong passwords for metrics authentication
   - Regularly rotate credentials

2. **Network Security**
   - Keep firewall rules minimal and specific
   - Monitor access logs regularly
   - Consider implementing rate limiting

3. **Updates**
   - Regularly update system packages
   - Keep IPFS and Hippius binaries up to date
   - Monitor security advisories

## Additional Resources

- [IPFS Documentation](https://docs.ipfs.tech/)
- [Caddy Documentation](https://caddyserver.com/docs/)
- [UFW Guide](https://help.ubuntu.com/community/UFW)
- [Systemd Documentation](https://www.freedesktop.org/software/systemd/man/)
