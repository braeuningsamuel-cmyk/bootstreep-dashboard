# Atlas.Lab Dashboard

Professional Tauri 2.x Desktop App für Homelab Server Management.

## Features

- **Dashboard**: CPU, RAM, Disk, Uptime, Network, Docker Container Übersicht
- **Docker**: Container Management (Start/Stop/Restart/Logs/Exec)
- **Services**: systemd Service Management
- **Terminal**: Lokale Shell mit History
- **Logs**: System-Logs (journalctl)
- **Ports**: Port-Check für 21 Homelab-Services
- **Files**: Datei-Explorer mit Editor
- **Network**: Interfaces, Routes, DNS, Firewall
- **Storage**: Mounts & Disk Usage
- **Processes**: Top Prozesse mit Kill
- **Crontab**: Cron-Job Editor
- **Packages**: Paketmanager Updates
- **Users**: Systembenutzer
- **Power**: Reboot / Shutdown
- **SSH Remote**: Verbindung zu Remote-Servern

## Tech Stack

- **Tauri 2.x** (Rust + WebView2)
- **Frontend**: Vanilla HTML/CSS/JS (kein Framework)
- **Theme**: Atlas.Lab Dark (#0a0a0f / #6366f1 / #10b981)
- **System**: sysinfo 0.33 (CPU, RAM, Disk, Network)
- **Shell**: tauri-plugin-shell 2

## Voraussetzungen

- **Node.js** 20+
- **Rust** 1.75+ (MSVC Toolchain via Visual Studio)
- **Windows 10/11** (WebView2 Runtime)

## Entwicklung

```bash
# Dependencies installieren
npm install

# Development Server starten (Hot Reload)
npm run tauri dev

# Production Build
npm run tauri build
```

## VS Code

Empfohlene Extensions werden automatisch vorgeschlagen (`.vscode/extensions.json`).

### Debugging

1. **Full Stack Debug** (F5) → startet Dev Server + Chrome Debugger
2. **Debug Atlas.Lab Dashboard (Rust)** → LLDB Debugger für Rust Backend
3. **Debug Frontend (Chrome)** → nur Frontend Debugging

### Tasks (Ctrl+Shift+P → Tasks)

- `tauri: dev` - Dev Server
- `tauri: build` - Production Build
- `cargo: check` - Rust Type Check
- `cargo: clippy` - Linting
- `cargo: fmt` - Formatting

## Projektstruktur

```
AtlasLab-Dashboard/
├── .vscode/              # VS Code Config
│   ├── launch.json       # Debug Configs
│   ├── tasks.json        # Build Tasks
│   ├── settings.json     # Editor Settings
│   └── extensions.json   # Empfohlene Extensions
├── src/                  # Frontend
│   ├── index.html        # Single-File SPA (HTML + CSS + JS)
│   └── main.js           # App Logic
├── src-tauri/            # Rust Backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── capabilities/     # Tauri 2.x Permissions
│   │   └── default.json
│   ├── icons/            # App Icons
│   └── src/
│       ├── lib.rs        # Tauri Commands & Logic
│       └── main.rs       # Entry Point
└── package.json
```

## Build Output

Nach `npm run tauri build`:

- `src-tauri/target/release/bundle/nsis/Atlas.Lab_2.0.0_x64-setup.exe` (~2 MB)
- `src-tauri/target/release/bundle/msi/Atlas.Lab_2.0.0_x64_en-US.msi` (~3 MB)

## SSH Remote Zugriff

1. Server-IP & User in Topbar eingeben
2. "Verbinden" klicken (SSH Key Auth vorausgesetzt)
3. App wechselt in Remote-Modus (lila Badge)
4. Alle Commands laufen auf dem Remote-Server

## Homelab Port-Mapping (Ports Tab)

| Port | Service |
|------|---------|
| 22 | SSH |
| 53 | Pi-hole DNS |
| 80/443 | HTTP/HTTPS (Caddy) |
| 3000 | Hermes |
| 3001 | Uptime Kuma |
| 445 | Samba |
| 51820 | WireGuard |
| 8080 | Websurfx |
| 8081 | Pi-hole Web |
| 8082/9443 | Nextcloud AIO |
| 8087 | AMP |
| 8096 | Jellyfin |
| 8384 | Syncthing |
| 8989/7878/9696/6767 | Arr Stack |
| 9050 | Tor |
| 11434 | Ollama |

## Lizenz

MIT