# Security Policy

## Supported Features and Security Measures

### Network Security

1. **P2P Communication**
   - Noise protocol encryption for all P2P connections
   - Secure peer identity using Ed25519 keypairs
   - Persistent peer IDs stored securely in the data directory
   - Automatic transport protocol negotiation

2. **WebSocket Security**
   - Secure WebSocket signaling server
   - Connection validation and peer authentication
   - Rate limiting on signaling messages
   - Automatic connection cleanup for inactive peers

3. **Monitoring and Metrics**
   - Basic authentication for metrics endpoint
   - Prometheus metrics exposed on a separate port
   - Resource usage monitoring and alerts
   - Connection statistics and peer tracking

### Access Control

1. **Node Authentication**
   - Unique peer IDs based on Ed25519 keypairs
   - Persistent peer identity across restarts
   - Bootnode verification for network entry

2. **Metrics Access**
   - Basic authentication required for metrics endpoint
   - Configurable through Caddy reverse proxy
   - Separate port for metrics collection (9091)

3. **Web Interface**
   - Served over HTTP/HTTPS via Caddy
   - Security headers for XSS protection
   - CORS configuration for API endpoints

## Security Best Practices

### Deployment

1. **Network Configuration**
   ```bash
   # Required open ports
   - 3000: Web interface
   - 8001: WebSocket signaling
   - 4002: P2P bootnode
   - 9091: Metrics server
   ```

2. **Reverse Proxy Setup**
   - Use Caddy for TLS termination
   - Enable security headers
   - Configure basic authentication for metrics
   - Set up rate limiting for endpoints

3. **File Permissions**
   ```bash
   # Data directory permissions
   chmod 700 /var/lib/hippius
   chown -R hippius:hippius /var/lib/hippius

   # Binary permissions
   chmod 755 /opt/hippius/hippius-libp2p
   chown root:root /opt/hippius/hippius-libp2p
   ```

### Monitoring

1. **Metrics Collection**
   - Monitor peer connections
   - Track resource usage
   - Watch for unusual network patterns
   - Set up alerts for anomalies

2. **Logging**
   - Enable debug logging for troubleshooting
   - Monitor connection attempts
   - Track peer behavior
   - Log security-related events

### Updates and Maintenance

1. **Regular Updates**
   - Keep the binary up to date
   - Update system dependencies
   - Monitor security advisories
   - Test updates in staging first

2. **Backup Procedures**
   - Backup peer identity keys
   - Secure storage of backups
   - Regular testing of restore procedures

## Reporting Security Issues

If you discover a security vulnerability in Hippius LibP2P, please follow these steps:

1. **Do Not** disclose the vulnerability publicly
2. Send a detailed report to security@thenervelab.com
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fixes (if any)

We will acknowledge receipt within 24 hours and provide a detailed response within 72 hours.

## Security Checklist

### Initial Setup
- [ ] Generate unique peer identity
- [ ] Configure secure file permissions
- [ ] Set up metrics authentication
- [ ] Configure firewall rules
- [ ] Set up reverse proxy with TLS

### Regular Maintenance
- [ ] Monitor system logs
- [ ] Check peer connections
- [ ] Review metrics data
- [ ] Update system packages
- [ ] Backup peer identity keys

### Emergency Response
- [ ] Document incident
- [ ] Isolate affected components
- [ ] Apply necessary patches
- [ ] Review security measures
- [ ] Update documentation

## Known Limitations

1. **Network**
   - No built-in TLS for direct connections (use reverse proxy)
   - Basic peer discovery mechanism
   - Limited DDoS protection

2. **Authentication**
   - Basic authentication for metrics only
   - No user authentication system
   - Peer IDs are the only form of identity

3. **Monitoring**
   - Basic metrics collection
   - No built-in alerting system
   - Limited historical data

## Future Security Enhancements

1. **Planned**
   - Enhanced peer authentication
   - Improved DDoS protection
   - Advanced metrics and monitoring
   - Better rate limiting

2. **Under Consideration**
   - Built-in TLS support
   - Advanced access control
   - Automated security scanning
   - Enhanced logging system
