# Release Process

This document describes the release process for Hippius LibP2P.

## Release Workflow

Our release process is automated using GitHub Actions. When you push a tag with the prefix `v`, it triggers the release workflow that builds binaries for multiple platforms and creates a GitHub release.

### Supported Platforms

The release workflow builds binaries for:
- Linux x86_64
- Linux ARM64
- macOS x86_64
- macOS ARM64 (Apple Silicon)

### Creating a New Release

1. **Update Version**
   ```bash
   # In Cargo.toml
   version = "X.Y.Z"  # Update this to the new version
   ```

2. **Commit Changes**
   ```bash
   git add Cargo.toml
   git commit -m "chore: bump version to vX.Y.Z"
   ```

3. **Create and Push Tag**
   ```bash
   git tag -a vX.Y.Z -m "Release version X.Y.Z"
   git push origin main
   git push origin vX.Y.Z
   ```

4. **Monitor Release**
   - Go to GitHub Actions tab to monitor the build process
   - Once completed, the release will be available in the Releases section

### Release Artifacts

Each release includes:
- Binary for each supported platform
- SHA256 checksums file
- Automatically generated release notes

### Verifying Release

To verify a release binary:
```bash
# Download the binary and checksums
wget https://github.com/your-org/hippius-libp2p/releases/download/vX.Y.Z/hippius-libp2p-{platform}
wget https://github.com/your-org/hippius-libp2p/releases/download/vX.Y.Z/checksums.txt

# Verify checksum
sha256sum -c checksums.txt
```

### Version Naming Convention

We follow [Semantic Versioning](https://semver.org/):
- MAJOR version for incompatible API changes
- MINOR version for backwards-compatible functionality
- PATCH version for backwards-compatible bug fixes

### Release Notes Guidelines

When creating a release tag, include relevant information in the tag message:
- New features
- Bug fixes
- Breaking changes
- Deprecations
- Performance improvements
- Dependencies updates

Example tag message:
```
Release version X.Y.Z

Features:
- Added new WebRTC signaling protocol
- Improved DDoS protection

Bug Fixes:
- Fixed memory leak in peer connection
- Resolved race condition in message handling

Breaking Changes:
- Changed API endpoint structure for better security
```

### Hotfix Releases

For urgent fixes:
1. Create a branch from the release tag
2. Make the fix
3. Create a new patch version
4. Follow the normal release process

### Release Channels

- **Stable**: Tagged releases (vX.Y.Z)
- **Development**: Latest main branch
- **Release Candidates**: vX.Y.Z-rc.N tags

### Post-Release

After a successful release:
1. Update documentation if needed
2. Announce the release in appropriate channels
3. Monitor for any reported issues
4. Update the changelog if necessary
