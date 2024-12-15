# Setting Up a TURN Server

This guide explains how to set up and configure a TURN server for our WebRTC application.

## Prerequisites

- A Linux server (Ubuntu 20.04 LTS recommended)
- Public IP address
- Domain name (recommended)
- SSL certificate (recommended for production)

## 1. Install Coturn

```bash
# Update package list
sudo apt-get update

# Install coturn
sudo apt-get install coturn
```

## 2. Configure Coturn

1. Create a configuration file:

```bash
sudo nano /etc/turnserver.conf
```

2. Add the following configuration:

```conf
# Basic configuration
listening-port=3478
tls-listening-port=5349

# Replace with your server's public IP
external-ip=YOUR_SERVER_IP

# Authentication
user=your-username:your-password
realm=your-domain.com

# TLS configuration (for production)
cert=/path/to/cert.pem
pkey=/path/to/key.pem

# Performance
total-quota=100
max-bps=0
user-quota=10
cli-password="your-cli-password"

# Logging
verbose
log-file=/var/log/turnserver.log
```

3. Enable the service:

```bash
sudo systemctl enable coturn
sudo systemctl start coturn
```

## 3. Configure Firewall

```bash
# Allow TURN server ports
sudo ufw allow 3478/tcp
sudo ufw allow 3478/udp
sudo ufw allow 5349/tcp
sudo ufw allow 5349/udp
```

## 4. Test the Server

1. Use trickle-ice to test:
   - Visit: https://webrtc.github.io/samples/src/content/peerconnection/trickle-ice/
   - Add your TURN server:
     ```
     turn:your-domain.com:3478
     username
     password
     ```

## 5. Monitoring

### Bandwidth Monitoring

1. Install monitoring tools:
```bash
sudo apt-get install vnstat
```

2. Monitor bandwidth:
```bash
# Real-time monitoring
vnstat -l

# Daily summary
vnstat -d
```

### Log Analysis

1. View TURN server logs:
```bash
tail -f /var/log/turnserver.log
```

2. Parse bandwidth usage:
```bash
grep "bandwidth" /var/log/turnserver.log | awk '{print $1, $2, $7}'
```

## 6. Production Considerations

1. **High Availability**:
   - Set up multiple TURN servers
   - Use DNS round-robin or load balancer
   - Consider geographic distribution

2. **Security**:
   - Use strong passwords
   - Enable TLS
   - Implement rate limiting
   - Monitor for abuse

3. **Performance**:
   - Monitor server resources
   - Set appropriate quotas
   - Consider bandwidth costs

4. **Maintenance**:
   - Regular security updates
   - Log rotation
   - Backup configuration
   - Monitor SSL certificate expiration

## 7. Updating Application Configuration

1. Update `config/turn_config.json`:
```json
{
    "turn": {
        "urls": ["turn:your-domain.com:3478"],
        "username": "your-username",
        "credential": "your-password"
    }
}
```

2. For production, force TURN usage:
```javascript
iceTransportPolicy: 'relay'  // Forces TURN usage
```

## Troubleshooting

1. **Connection Issues**:
   - Check firewall rules
   - Verify server is running: `systemctl status coturn`
   - Check logs: `tail -f /var/log/turnserver.log`

2. **Performance Issues**:
   - Monitor server load: `top`, `htop`
   - Check bandwidth: `vnstat -l`
   - Review connection logs

3. **Authentication Issues**:
   - Verify credentials in config
   - Check realm setting
   - Review client-side configuration
