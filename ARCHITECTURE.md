# Bootstreep Dashboard — Architecture

## Overview

Bootstreep Dashboard is a web‑based homelab control panel. The legacy version
was a Tauri 2.x desktop app; the active line of development is a **cross‑
platform web app** (Flask backend + vanilla‑JS frontend) that runs anywhere
Python 3.11+ and a browser are available — Windows, macOS, Linux.

This document describes the web‑app architecture, the legacy Tauri architecture
(for historical context), and the migration path between the two.

## Goals

- **Zero install** — open the URL, see your homelab.
- **OS‑agnostic** — no native binaries, no platform‑specific UI.
- **Small surface** — Flask backend, single‑page vanilla‑JS frontend, no
  build step.
- **Secure by default** — the backend exposes only narrow, validated APIs; the
  remote‑host agent is a separate Rust binary with an explicit allow‑list.

## High‑level diagram

```
                +---------------------------------------+
                |            Web browser               |
                |   (Chromium / Firefox / Safari)       |
                +-------------------+-------------------+
                                    | HTTPS
                                    v
                +---------------------------------------+
                |         Flask backend (Python)        |
                |  /api/system_stats  /api/repos        |
                |  /api/health        /api/repos/<n>/fetch|
                +---+----------+-------------+---------+
                    |          |             |
        +-----------+          |             +-----------+
        |                      |                         |
        v                      v                         v
+-----------------+   +-------------------+   +-------------------+
|   psutil        |   |   GitPython       |   |   local repos     |
| (CPU/RAM/Disk)  |   | (per‑repo status) |   |   ./repos/<name>  |
+-----------------+   +-------------------+   +-------------------+
```

## Components

### 1. Flask backend (`backend/app.py`)

Single‑module Flask app. Exposes:

| Endpoint                 | Purpose                                                   |
|--------------------------|-----------------------------------------------------------|
| `GET /`                  | Serves `frontend/templates/index.html`                    |
| `GET /static/<file>`     | Serves CSS/JS under `frontend/static/`                    |
| `GET /api/health`        | Liveness probe — returns `{"status":"ok"}`                |
| `GET /api/system_stats`  | psutil snapshot — CPU%, memory, disk, network, uptime     |
| `GET /api/repos`         | Per‑repo status (branch, last commit, behind/ahead, quality score) |
| `POST /api/repos/<n>/fetch` | Force `git fetch` for one repo                       |

The backend reads the local clones under `~/web-dashboard/repos/` using
`GitPython`. It never mutates the working copy.

### 2. Frontend (`frontend/`)

Pure vanilla JS — no build, no bundler, no npm dependencies at runtime.

- **`templates/index.html`** — single page with three sections:
  System stats, Repositories, Quality Bar.
- **`static/css/style.css`** — dark‑themed responsive layout, CSS variables.
- **`static/js/main.js`** — fetches `/api/system_stats` and `/api/repos`,
  renders the cards, auto‑refreshes every 60s, exposes a manual refresh
  button.

### 3. Local repo clones (`repos/`)

Each GitHub repo is mirrored locally under `~/web-dashboard/repos/<name>/`.
The backend uses `GitPython` to:

- read `active_branch` and `head.commit`
- compare against `origin/<branch>` to compute `behind` / `ahead`
- score the **Quality Bar** (see below)

## Quality Bar

Five‑point score per repo, surfaced in the dashboard:

1. **Makefile** — root `Makefile` with `help`, `lint`, `test` targets
2. **ARCHITECTURE.md** — root `ARCHITECTURE.md` ≥ 100 chars
3. **.env.example** — root `.env.example` documenting required env vars
4. **CI workflow** — `.github/workflows/*.yml` exists
5. **Dockerfiles / compose** — any tracked Dockerfile or `docker-compose*.yml`

A sixth soft signal is **BATS tests** count, where present.

The score is computed on every `/api/repos` call — no caching, so the
dashboard always reflects the current working copy.

## Legacy Tauri architecture (preserved)

The `src-tauri/` directory contains the legacy Tauri 2.x shell. It is kept
for reference and for users who still prefer a native desktop install.

```
src-tauri/
|-- build.rs
|-- Cargo.toml            # Rust deps
|-- capabilities/         # Tauri 2.x permission ACL
|-- src/
|   |-- lib.rs            # public Tauri commands
|   |-- main.rs           # process entry point
|   `-- path_sandbox.rs   # hardened path sandboxing
`-- tauri.conf.json
```

The frontend HTML/JS under `src/` is the pre‑web port of the same dashboard
logic — most of the system‑stats and repo‑status code now lives in
`backend/app.py` instead of Tauri commands.

## Security model

### Backend

- Binds `0.0.0.0` so it can be reached from other devices on the LAN; consider
  reverse‑proxying with TLS (Caddy, nginx) before exposing beyond localhost.
- No authentication is built in — deploy behind SSO or basic auth at the
  proxy layer.
- No command execution from the frontend. The backend is read‑only with
  respect to the repos; the only write is `git fetch`, which is idempotent.

### Frontend

- No `eval`, no inline scripts. All JS is served from `/static/`.
- No third‑party trackers or analytics.
- XSS‑safe by construction: all dynamic strings are inserted via
  `textContent` or escaped with a small helper in `main.js`.

### CI workflow

- Pinned action SHAs (e.g. `actions/checkout@v4.2.2`).
- Network egress limited to `pip` and `apt`.
- Web‑dashboard CI does a `py_compile` smoke test plus a `curl /api/health`
  probe against a locally‑started Flask process.

## Deployment

The simplest deployment is:

```bash
cd ~/web-dashboard/backend
source venv/Scripts/activate        # Windows
# source venv/bin/activate          # macOS / Linux
python app.py                       # listens on 0.0.0.0:5000
```

Open `http://<host>:5000` from any device on the same network.

For a hardened deployment, place the Flask process behind a reverse proxy:

```
internet ── TLS ──> Caddy (port 443)
                       |
                       └──> Flask (127.0.0.1:5000)
```

## Extension points

- **Add a new endpoint** → drop a `@app.route('/api/<name>')` in `backend/app.py`.
- **Add a new section to the UI** → extend `frontend/templates/index.html`
  + `static/js/main.js`; no rebuild step required.
- **Add a new repo to track** → `git clone` it under `~/web-dashboard/repos/`;
  it will appear automatically on the next `/api/repos` call.

## Versioning

This dashboard tracks `semver`. The current version is declared in
`package.json` and surfaced in the dashboard footer.