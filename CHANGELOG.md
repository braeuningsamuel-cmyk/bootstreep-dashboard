# Changelog

## v2.1.0 (2026-06-13)

### Features
- Homelab service URL table on the Homelab page (all 22 services with ports and links)
- Extended port check list: +5 ports (3002 Open WebUI, 5678 n8n, 8085 SABnzbd, 8090 Heimdall, 51821 PiVPN)

## v2.0.0 (2026-06-12)

### Features
- Multi-server profile management (add/remove/switch profiles with dropdown)
- Theme toggle: Dark, Light, System (auto-detection)
- Live metrics via Tauri events (real-time CPU/RAM in topbar)
- PTY terminal with full shell session support
- Tauri 2.x migration

### Homelab Integrations
- WireGuard peer management
- Jellyfin media server control
- Arr-Stack (Sonarr/Radarr/Lidarr) status
- Ollama model listing
- Syncthing folder status
- Uptime Kuma monitoring status
- Nextcloud OCC command execution

### Infrastructure
- Event delegation architecture for CSP compliance
- Mobile-responsive design with bottom navigation
- File explorer with editor (read/write server files)
- Process viewer with kill capability
- System power controls (reboot, shutdown)

## v1.0.0 (2026-06-01)

Initial release.
- Basic server dashboard with system stats
- Docker container management (list, start, stop, restart, logs)
- Service management (systemd)
- Network information, firewall control
- Storage monitoring
- Port checking
- Settings page for connection configuration
