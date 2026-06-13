const invoke = window.__TAURI__?.core?.invoke ?? (async () => {
  throw new Error('Tauri bridge unavailable. Enable app.withGlobalTauri in tauri.conf.json or switch to the @tauri-apps/api/core import.');
});
let currentPage = 'dashboard';
let pollInterval = null;
let connected = false;
let dockerData = [];
let serviceData = [];
let procData = [];
let currentFilePath = '/';
let editingFilePath = '';
let termHistory = [];
let termHistIdx = -1;

// Sparkline data
const sparkData = { cpu: [], mem: [] };
const SPARK_MAX = 30;

// ── Utilities ──

function esc(s) { const d = document.createElement('div'); d.textContent = s; return d.innerHTML; }

function toast(msg, type = 'ok') {
  const el = document.createElement('div');
  el.className = 'toast ' + type;
  el.textContent = msg;
  document.getElementById('toast-wrap').appendChild(el);
  setTimeout(() => el.remove(), 3500);
}

function formatBytes(b) {
  if (!b || b === 0) return '0 B';
  const k = 1024, s = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(b) / Math.log(k));
  return (b / Math.pow(k, i)).toFixed(1) + ' ' + s[i];
}

function formatUptime(s) {
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
  if (d > 0) return d + 'd ' + h + 'h';
  if (h > 0) return h + 'h ' + m + 'm';
  return m + 'm';
}

function barColor(pct) { return pct > 90 ? 'bar-red' : pct > 70 ? 'bar-yellow' : 'bar-green'; }

function sparkline(container, data, color) {
  if (!container || data.length < 2) return;
  const w = container.offsetWidth || 200, h = 40;
  const max = Math.max(...data, 1);
  const pts = data.map((v, i) => `${(i / (data.length - 1)) * w},${h - (v / max) * (h - 2)}`).join(' ');
  container.innerHTML = `<svg width="${w}" height="${h}" viewBox="0 0 ${w} ${h}"><defs><linearGradient id="sg-${color.replace('#', '')}" x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stop-color="${color}" stop-opacity=".25"/><stop offset="100%" stop-color="${color}" stop-opacity="0"/></linearGradient></defs><polygon fill="url(#sg-${color.replace('#', '')})" points="0,${h} ${pts} ${w},${h}" opacity=".8"/><polyline fill="none" stroke="${color}" stroke-width="1.5" points="${pts}"/></svg>`;
}

// ── Dialog ──

function showDialog(title, message, icon, onConfirm, dangerous = false) {
  document.getElementById('dialog-title').textContent = title;
  document.getElementById('dialog-message').textContent = message;
  document.getElementById('dialog-icon').textContent = icon;
  const btn = document.getElementById('dialog-confirm');
  btn.className = 'dialog-btn confirm' + (dangerous ? ' danger' : '');
  btn.onclick = () => { closeDialog(); onConfirm(); };
  document.getElementById('dialog-overlay').classList.add('on');
}

function closeDialog() { document.getElementById('dialog-overlay').classList.remove('on'); }

// ── Navigation ──

const pageTitles = {
  dashboard: 'Dashboard', docker: 'Docker', services: 'Services', files: 'Dateien',
  network: 'Netzwerk', storage: 'Speicher', processes: 'Prozesse', packages: 'Pakete',
  users: 'Benutzer', crontab: 'Crontab', terminal: 'Terminal', logs: 'Logs', ports: 'Ports', power: 'Power'
};

function showPage(name) {
  document.querySelectorAll('.page').forEach(p => p.classList.remove('on'));
  document.querySelectorAll('.sb-i,.bn-i').forEach(b => b.classList.remove('on'));
  const page = document.getElementById('page-' + name);
  if (page) page.classList.add('on');
  document.querySelectorAll(`[data-page="${name}"]`).forEach(b => b.classList.add('on'));
  document.getElementById('tb-title').textContent = pageTitles[name] || name;
  currentPage = name;
  toggleMore(true);

  // Lazy load data
  const loaders = {
    docker: loadDocker, services: loadServices, files: loadFiles, network: loadNetwork,
    storage: loadStorage, processes: loadProcesses, packages: () => { }, users: loadUsers,
    crontab: loadCrontab, ports: checkPorts, terminal: () => document.getElementById('term-input').focus()
  };
  if (loaders[name]) loaders[name]();
}

// Sidebar & bottom nav click handlers
document.querySelectorAll('.sb-i[data-page],.bn-i[data-page],.more-sheet button[data-page]').forEach(btn => {
  btn.addEventListener('click', () => showPage(btn.dataset.page));
});

window.toggleMore = function toggleMore(forceClose) {
  const o = document.getElementById('more-overlay');
  const s = document.getElementById('more-sheet');
  if (forceClose === true || o.classList.contains('on')) {
    o.classList.remove('on');
    s.classList.remove('on');
  } else {
    o.classList.add('on');
    s.classList.add('on');
  }
}

// ── Connection ──

function updateModeBadge(mode, host) {
  const badge = document.getElementById('mode-badge');
  if (mode === 'remote' && host) {
    badge.textContent = 'Remote: ' + host;
    badge.classList.add('remote');
  } else {
    badge.textContent = 'Lokal';
    badge.classList.remove('remote');
  }
}

async function applyConnection() {
  const host = (document.getElementById('host-input').value || document.getElementById('m-host')?.value || '').trim();
  const user = (document.getElementById('user-input').value || document.getElementById('m-user')?.value || '').trim();
  localStorage.setItem('atlaslab-host', host);
  localStorage.setItem('atlaslab-user', user);
  const info = await invoke('set_connection', { host, user });
  updateModeBadge(info.mode, info.host);
  return info;
}

window.testConnection = async function testConnection() {
  const dot = document.getElementById('conn-dot');
  const btn = document.getElementById('conn-btn');
  dot.classList.remove('ok');
  btn.disabled = true; btn.textContent = '…';
  try {
    await applyConnection();
    const ok = await invoke('test_ssh_connection');
    if (ok) {
      dot.classList.add('ok');
      connected = true;
      document.getElementById('sb-conn-label').textContent = 'Verbunden';
      toast('Verbindung erfolgreich');
      loadDashboard();
    } else {
      connected = false;
      document.getElementById('sb-conn-label').textContent = 'Fehler';
      toast('Verbindung fehlgeschlagen', 'err');
    }
  } catch (e) {
    connected = false;
    dot.classList.remove('ok');
    document.getElementById('sb-conn-label').textContent = 'Fehler';
    toast('Fehler: ' + e, 'err');
  }
  btn.disabled = false; btn.textContent = 'Verbinden';
}

// ── Dashboard ──

function setDashError(msg) {
  document.getElementById('d-cpu').textContent = 'Fehler';
  document.getElementById('d-cpu-sub').textContent = msg;
  document.getElementById('d-mem').textContent = '—';
  document.getElementById('d-disk').textContent = '—';
  document.getElementById('d-uptime').textContent = '—';
  document.getElementById('d-net-rx').textContent = '—';
  document.getElementById('d-net-tx').textContent = '—';
  document.getElementById('d-host').textContent = '—';
  document.getElementById('d-os').textContent = '—';
  document.getElementById('d-containers').innerHTML = `<tr><td colspan="4" style="color:var(--red)">${esc(msg)}</td></tr>`;
  document.getElementById('tb-cpu').textContent = '—';
  document.getElementById('tb-mem').textContent = '—';
  const u = document.getElementById('dash-updated');
  if (u) u.textContent = 'Fehler: ' + new Date().toLocaleTimeString('de-DE');
}

async function loadDashboard() {
  try {
    const s = await invoke('system_stats');
    document.getElementById('d-cpu').textContent = s.cpu_usage.toFixed(1) + '%';
    document.getElementById('d-cpu-sub').textContent = s.hostname;
    document.getElementById('d-cpu-bar').style.width = s.cpu_usage + '%';
    document.getElementById('d-cpu-bar').className = 'card-bar-fill ' + barColor(s.cpu_usage);
    document.getElementById('d-mem').textContent = s.mem_percent.toFixed(1) + '%';
    document.getElementById('d-mem-sub').textContent = formatBytes(s.mem_used) + ' / ' + formatBytes(s.mem_total);
    document.getElementById('d-mem-bar').style.width = s.mem_percent + '%';
    document.getElementById('d-mem-bar').className = 'card-bar-fill ' + barColor(s.mem_percent);
    document.getElementById('d-disk').textContent = s.disk_percent.toFixed(1) + '%';
    document.getElementById('d-disk-sub').textContent = formatBytes(s.disk_used) + ' / ' + formatBytes(s.disk_total);
    document.getElementById('d-disk-bar').style.width = s.disk_percent + '%';
    document.getElementById('d-disk-bar').className = 'card-bar-fill ' + barColor(s.disk_percent);
    document.getElementById('d-uptime').textContent = formatUptime(s.uptime);
    document.getElementById('d-uptime-sub').textContent = 'Seit ' + new Date(Date.now() - s.uptime * 1000).toLocaleDateString('de-DE');
    document.getElementById('d-net-rx').textContent = formatBytes(s.network_rx);
    document.getElementById('d-net-tx').textContent = formatBytes(s.network_tx);
    document.getElementById('d-host').textContent = s.hostname;
    document.getElementById('d-os').textContent = s.os;
    document.getElementById('tb-cpu').textContent = s.cpu_usage.toFixed(1) + '%';
    document.getElementById('tb-mem').textContent = s.mem_percent.toFixed(1) + '%';
    const u = document.getElementById('dash-updated');
    if (u) u.textContent = 'Aktualisiert: ' + new Date().toLocaleTimeString('de-DE');

    // Sparklines
    sparkData.cpu.push(s.cpu_usage); if (sparkData.cpu.length > SPARK_MAX) sparkData.cpu.shift();
    sparkData.mem.push(s.mem_percent); if (sparkData.mem.length > SPARK_MAX) sparkData.mem.shift();
    sparkline(document.getElementById('spark-cpu'), sparkData.cpu, '#6366f1');
    sparkline(document.getElementById('spark-mem'), sparkData.mem, '#10b981');

    // Dashboard containers
    try {
      const containers = await invoke('docker_list');
      let html = '';
      containers.slice(0, 8).forEach(c => {
        html += `<tr><td>${esc(c.name)}</td><td><span class="badge badge-${c.state === 'running' ? 'running' : 'stopped'}">${esc(c.status)}</span></td><td style="font-size:11px;color:var(--text3)">${esc(c.image)}</td><td style="font-size:11px;color:var(--text3)">${esc(c.ports || '-')}</td></tr>`;
      });
      document.getElementById('d-containers').innerHTML = html || '<tr><td colspan="4" style="color:var(--text3)">Keine Container</td></tr>';
    } catch (e) {
      document.getElementById('d-containers').innerHTML = '<tr><td colspan="4" style="color:var(--text3)">Docker nicht verfügbar</td></tr>';
    }
  } catch (e) {
    setDashError('System Stats nicht verfügbar: ' + e);
    toast('Dashboard: ' + e, 'err');
  }
}

// ── Docker ──

function renderDockerTable(data) {
  let html = '';
  data.forEach(c => {
    const r = c.state === 'running';
    html += `<tr><td>${esc(c.name)}</td><td><span class="badge badge-${r ? 'running' : 'stopped'}">${esc(c.status)}</span></td><td style="font-size:11px;color:var(--text3)">${esc(c.image)}</td><td style="font-size:11px;color:var(--text3)">${esc(c.ports || '-')}</td><td style="white-space:nowrap">`;
    if (r) {
      html += `<button class="action-btn" data-da="stop" data-n="${esc(c.name)}">Stop</button> <button class="action-btn success" data-da="restart" data-n="${esc(c.name)}">Restart</button>`;
    } else {
      html += `<button class="action-btn success" data-da="start" data-n="${esc(c.name)}">Start</button>`;
    }
    html += ` <button class="action-btn" data-dl="${esc(c.name)}">Logs</button></td></tr>`;
  });
  document.getElementById('docker-table').innerHTML = html || '<tr><td colspan="5" style="color:var(--text3)">Keine Container</td></tr>';
}

window.filterDocker = function filterDocker() {
  const q = document.getElementById('docker-search').value.toLowerCase();
  renderDockerTable(dockerData.filter(c => c.name.toLowerCase().includes(q) || c.image.toLowerCase().includes(q)));
}

window.loadDocker = async function loadDocker() {
  try {
    dockerData = await invoke('docker_list');
    filterDocker();
    // Load stats too
    try {
      const stats = await invoke('docker_stats');
      let html = '';
      stats.forEach(s => {
        html += `<tr><td>${esc(s.name)}</td><td>${esc(s.cpu_percent)}</td><td>${esc(s.mem_usage)}</td><td>${esc(s.net_io)}</td><td>${esc(s.block_io)}</td><td>${esc(s.pids)}</td></tr>`;
      });
      document.getElementById('docker-stats-table').innerHTML = html || '<tr><td colspan="6" style="color:var(--text3)">Keine laufenden Container</td></tr>';
    } catch (e) { }
  } catch (e) {
    document.getElementById('docker-table').innerHTML = `<tr><td colspan="5" style="color:var(--red)">Fehler: ${esc(String(e))}</td></tr>`;
  }
}

async function dockerAct(action, name) {
  try { await invoke('docker_action', { action, name }); toast(name + ': ' + action + ' OK'); setTimeout(loadDocker, 500); }
  catch (e) { toast('Fehler: ' + e, 'err'); }
}

window.loadLogs = async function loadLogs() {
  const name = document.getElementById('log-container').value.trim();
  if (!name) return;
  try {
    const logs = await invoke('docker_logs', { name, lines: 100 });
    document.getElementById('log-output').textContent = logs || '(leer)';
    document.getElementById('log-output').scrollTop = 99999;
  } catch (e) { document.getElementById('log-output').textContent = 'Fehler: ' + e; }
}

document.getElementById('docker-table').addEventListener('click', e => {
  const a = e.target.closest('[data-da]');
  if (a) { dockerAct(a.dataset.da, a.dataset.n); return; }
  const l = e.target.closest('[data-dl]');
  if (l) { document.getElementById('log-container').value = l.dataset.dl; loadLogs(); }
});

// ── Services ──

function renderServiceTable(data) {
  let html = '';
  data.slice(0, 100).forEach(s => {
    const act = s.active === 'active' || s.status === 'running';
    html += `<tr><td>${esc(s.name)}</td><td><span class="badge badge-${act ? 'active' : 'inactive'}">${esc(s.status)}</span></td><td>${esc(s.active)}</td><td style="font-size:11px;color:var(--text3)">${esc(s.description)}</td><td style="white-space:nowrap"><button class="action-btn success" data-sa="restart" data-sn="${esc(s.name)}">Restart</button> <button class="action-btn" data-sa="stop" data-sn="${esc(s.name)}">Stop</button></td></tr>`;
  });
  document.getElementById('service-table').innerHTML = html || '<tr><td colspan="5" style="color:var(--text3)">Keine Services</td></tr>';
}

window.filterServices = function filterServices() {
  const q = document.getElementById('service-search').value.toLowerCase();
  renderServiceTable(serviceData.filter(s => s.name.toLowerCase().includes(q) || s.description.toLowerCase().includes(q)));
}

window.loadServices = async function loadServices() {
  try { serviceData = await invoke('service_list'); filterServices(); }
  catch (e) { document.getElementById('service-table').innerHTML = `<tr><td colspan="5" style="color:var(--red)">Fehler: ${esc(String(e))}</td></tr>`; }
}

document.getElementById('service-table').addEventListener('click', e => {
  const btn = e.target.closest('[data-sa]');
  if (btn) {
    const act = btn.dataset.sa, name = btn.dataset.sn;
    if (act === 'stop') {
      showDialog('Service stoppen', `"${name}" wirklich stoppen?`, '⚙️', () => serviceAct(act, name), true);
    } else { serviceAct(act, name); }
  }
});

async function serviceAct(action, name) {
  try { await invoke('service_action', { action, name }); toast(name + ': ' + action + ' OK'); setTimeout(loadServices, 500); }
  catch (e) { toast('Fehler: ' + e, 'err'); }
}

// ── File Explorer ──

function renderBreadcrumb() {
  const parts = currentFilePath.split('/').filter(Boolean);
  let html = '<button onclick="navigateFile(\'/\')">/</button>';
  let path = '';
  parts.forEach((p, i) => {
    path += '/' + p;
    html += `<span>/</span><button data-action="navigateFile" data-arg="${path}">${esc(p)}</button>`;
  });
  document.getElementById('file-breadcrumb').innerHTML = html;
}

window.navigateFile = function navigateFile(path) { currentFilePath = path; loadFiles(); }

window.loadFiles = async function loadFiles() {
  renderBreadcrumb();
  document.getElementById('file-editor-wrap').style.display = 'none';
  try {
    const files = await invoke('file_list', { path: currentFilePath });
    let html = '';
    // Parent directory
    if (currentFilePath !== '/') {
      const parent = currentFilePath.split('/').slice(0, -1).join('/') || '/';
      html += `<tr style="cursor:pointer" data-action="navigateFile" data-arg="${parent}"><td>📁</td><td>..</td><td></td><td></td><td></td><td></td></tr>`;
    }
    files.forEach(f => {
      const fp = currentFilePath === '/' ? '/' + f.name : currentFilePath + '/' + f.name;
      const icon = f.is_dir ? '📁' : '📄';
      html += `<tr>`;
      html += `<td>${icon}</td>`;
      if (f.is_dir) {
        html += `<td><a href="#" data-action="navigateFile" data-arg="${esc(fp)}" style="color:var(--accent2);text-decoration:none">${esc(f.name)}</a></td>`;
      } else {
        html += `<td>${esc(f.name)}</td>`;
      }
      html += `<td style="font-size:11px;color:var(--text3)">${f.is_dir ? '-' : formatBytes(f.size)}</td>`;
      html += `<td style="font-size:11px;color:var(--text3)">${esc(f.modified.substring(0, 19))}</td>`;
      html += `<td style="font-size:11px;font-family:var(--mono);color:var(--text3)">${esc(f.permissions)}</td>`;
      html += `<td style="white-space:nowrap">`;
      if (!f.is_dir) html += `<button class="action-btn" data-action="openFile" data-arg="${esc(fp)}">Öffnen</button> `;
      html += `<button class="action-btn danger" data-action="deleteFile" data-path="${esc(fp)}" data-name="${esc(f.name)}" data-isdir="${f.is_dir}">Löschen</button>`;
      html += `</td></tr>`;
    });
    document.getElementById('file-table').innerHTML = html || '<tr><td colspan="6" style="color:var(--text3)">Leerer Ordner</td></tr>';
  } catch (e) {
    document.getElementById('file-table').innerHTML = `<tr><td colspan="6" style="color:var(--red)">Fehler: ${esc(String(e))}</td></tr>`;
  }
}

window.openFile = async function openFile(path) {
  try {
    const content = await invoke('file_read', { path });
    editingFilePath = path;
    document.getElementById('file-editor-title').textContent = 'Datei: ' + path.split('/').pop();
    document.getElementById('file-editor-content').value = content;
    document.getElementById('file-editor-wrap').style.display = 'block';
  } catch (e) { toast('Fehler beim Öffnen: ' + e, 'err'); }
}

window.closeFileEditor = function closeFileEditor() { document.getElementById('file-editor-wrap').style.display = 'none'; editingFilePath = ''; }

window.saveFile = async function saveFile() {
  if (!editingFilePath) return;
  const content = document.getElementById('file-editor-content').value;
  try { await invoke('file_write', { path: editingFilePath, content }); toast('Gespeichert: ' + editingFilePath.split('/').pop()); }
  catch (e) { toast('Fehler: ' + e, 'err'); }
}

window.deleteFile = function deleteFile(path, name, isDir) {
  showDialog('Löschen', `"${name}" wirklich löschen?${isDir ? ' (Ordner + Inhalt!)' : ''}`, '🗑️', async () => {
    try { await invoke('file_delete', { path }); toast(name + ' gelöscht'); loadFiles(); }
    catch (e) { toast('Fehler: ' + e, 'err'); }
  }, true);
}

window.fileMkdir = function fileMkdir() {
  const name = prompt('Neuer Ordnername:');
  if (!name) return;
  const path = (currentFilePath === '/' ? '/' : currentFilePath + '/') + name;
  invoke('file_mkdir', { path }).then(() => { toast('Ordner erstellt'); loadFiles(); }).catch(e => toast('Fehler: ' + e, 'err'));
}

// ── Network ──

window.loadNetwork = async function loadNetwork() {
  try { const r = await invoke('network_info'); document.getElementById('network-output').textContent = r; }
  catch (e) { document.getElementById('network-output').textContent = 'Fehler: ' + e; }
  loadFirewall();
}

window.loadFirewall = async function loadFirewall() {
  try { const r = await invoke('firewall_status'); document.getElementById('firewall-output').textContent = r; }
  catch (e) { document.getElementById('firewall-output').textContent = 'Fehler: ' + e; }
}

window.fwAction = async function fwAction(action) {
  const rule = document.getElementById('fw-rule').value.trim();
  if (!rule) return toast('Regel eingeben', 'err');
  showDialog('Firewall', `${action.toUpperCase()} ${rule}?`, '🛡️', async () => {
    try { const r = await invoke('firewall_action', { action, rule }); toast('Firewall: ' + r); loadFirewall(); }
    catch (e) { toast('Fehler: ' + e, 'err'); }
  });
}

// ── Storage ──

window.loadStorage = async function loadStorage() {
  try {
    const mounts = await invoke('storage_info');
    // Cards
    let cards = '';
    mounts.slice(0, 6).forEach(m => {
      cards += `<div class="card"><div class="card-label">${esc(m.mount_point)}</div><div class="card-value">${m.percent.toFixed(1)}%</div><div class="card-sub">${formatBytes(m.used)} / ${formatBytes(m.total)}</div><div class="card-bar"><div class="card-bar-fill ${barColor(m.percent)}" style="width:${m.percent}%"></div></div></div>`;
    });
    document.getElementById('storage-cards').innerHTML = cards;
    // Table
    let html = '';
    mounts.forEach(m => {
      html += `<tr><td style="font-size:12px">${esc(m.filesystem)}</td><td>${esc(m.mount_point)}</td><td>${formatBytes(m.total)}</td><td>${formatBytes(m.used)}</td><td>${formatBytes(m.available)}</td><td><span class="badge badge-${m.percent > 90 ? 'stopped' : m.percent > 70 ? 'stopped' : 'active'}">${m.percent.toFixed(1)}%</span></td></tr>`;
    });
    document.getElementById('storage-table').innerHTML = html || '<tr><td colspan="6" style="color:var(--text3)">Keine Daten</td></tr>';
  } catch (e) {
    document.getElementById('storage-table').innerHTML = `<tr><td colspan="6" style="color:var(--red)">Fehler: ${esc(String(e))}</td></tr>`;
  }
}

// ── Processes ──

function renderProcTable(data) {
  let html = '';
  data.forEach(p => {
    html += `<tr><td style="font-family:var(--mono);font-size:12px">${p.pid}</td><td>${esc(p.user)}</td><td>${p.cpu.toFixed(1)}%</td><td>${typeof p.mem === 'number' && p.mem > 100 ? formatBytes(p.mem * 1024 * 1024) : p.mem.toFixed(1) + '%'}</td><td style="font-size:11px;color:var(--text3);max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${esc(p.command)}</td><td><button class="action-btn danger" data-action="killProc" data-arg="${p.pid}">Kill</button></td></tr>`;
  });
  document.getElementById('proc-table').innerHTML = html || '<tr><td colspan="6" style="color:var(--text3)">Keine Prozesse</td></tr>';
}

window.filterProcs = function filterProcs() {
  const q = document.getElementById('proc-search').value.toLowerCase();
  renderProcTable(procData.filter(p => p.command.toLowerCase().includes(q) || String(p.pid).includes(q) || p.user.toLowerCase().includes(q)));
}

window.loadProcesses = async function loadProcesses() {
  try { procData = await invoke('process_list'); filterProcs(); }
  catch (e) { document.getElementById('proc-table').innerHTML = `<tr><td colspan="6" style="color:var(--red)">Fehler: ${esc(String(e))}</td></tr>`; }
}

window.killProc = function killProc(pid) {
  showDialog('Prozess beenden', `Prozess ${pid} beenden?`, '💀', async () => {
    try { await invoke('process_kill', { pid, signal: 'TERM' }); toast('Prozess ' + pid + ' beendet'); setTimeout(loadProcesses, 500); }
    catch (e) { toast('Fehler: ' + e, 'err'); }
  }, true);
}

// ── Packages ──

window.loadPackages = async function loadPackages() {
  document.getElementById('pkg-output').textContent = 'Suche Updates…';
  try { const r = await invoke('package_updates'); document.getElementById('pkg-output').textContent = r || 'Alle Pakete aktuell.'; }
  catch (e) { document.getElementById('pkg-output').textContent = 'Fehler: ' + e; }
}

window.pkgAction = async function pkgAction(action) {
  const name = document.getElementById('pkg-name').value.trim();
  if (!name) return toast('Paketname eingeben', 'err');
  showDialog('Paket ' + (action === 'install' ? 'installieren' : 'entfernen'), `"${name}" ${action}?`, '📦', async () => {
    try { const r = await invoke('package_action', { name, action }); toast('OK: ' + name); document.getElementById('pkg-output').textContent = r; }
    catch (e) { toast('Fehler: ' + e, 'err'); }
  }, action === 'remove');
}

// ── Users ──

window.loadUsers = async function loadUsers() {
  try {
    const users = await invoke('user_list');
    let html = '';
    users.forEach(u => {
      html += `<tr><td>${esc(u.name)}</td><td style="font-family:var(--mono);font-size:12px">${u.uid}</td><td style="font-family:var(--mono);font-size:12px">${u.gid}</td><td style="font-size:12px">${esc(u.home)}</td><td style="font-size:12px;color:var(--text3)">${esc(u.shell)}</td><td style="font-size:12px;color:var(--text3)">${esc(u.info)}</td></tr>`;
    });
    document.getElementById('user-table').innerHTML = html || '<tr><td colspan="6" style="color:var(--text3)">Keine Benutzer</td></tr>';
  } catch (e) {
    document.getElementById('user-table').innerHTML = `<tr><td colspan="6" style="color:var(--red)">Fehler: ${esc(String(e))}</td></tr>`;
  }
}

// ── Crontab ──

window.loadCrontab = async function loadCrontab() {
  try { const r = await invoke('crontab_list'); document.getElementById('crontab-editor').value = r; }
  catch (e) { document.getElementById('crontab-editor').value = 'Fehler: ' + e; }
}

window.saveCrontab = async function saveCrontab() {
  const content = document.getElementById('crontab-editor').value;
  showDialog('Crontab speichern', 'Crontab wirklich überschreiben?', '⏰', async () => {
    try { await invoke('crontab_save', { content }); toast('Crontab gespeichert'); }
    catch (e) { toast('Fehler: ' + e, 'err'); }
  });
}

// ── Terminal ──

async function runTermCmd(cmd) {
  const out = document.getElementById('term-output');
  out.textContent += '$ ' + cmd + '\n';
  try { const r = await invoke('run_command', { command: cmd }); out.textContent += r + '\n\n'; }
  catch (e) { out.textContent += 'Fehler: ' + e + '\n\n'; }
  out.scrollTop = 99999;
}

document.getElementById('term-input').addEventListener('keydown', function (e) {
  if (e.key === 'Enter') { const cmd = this.value.trim(); if (cmd) { termHistory.push(cmd); termHistIdx = termHistory.length; runTermCmd(cmd); } this.value = ''; }
  if (e.key === 'ArrowUp') { e.preventDefault(); if (termHistIdx > 0) { termHistIdx--; this.value = termHistory[termHistIdx]; } }
  if (e.key === 'ArrowDown') { e.preventDefault(); if (termHistIdx < termHistory.length - 1) { termHistIdx++; this.value = termHistory[termHistIdx]; } else { termHistIdx = termHistory.length; this.value = ''; } }
});

// ── Logs ──

window.loadSysLog = async function loadSysLog() {
  const cmd = document.getElementById('syslog-cmd').value.trim();
  if (!cmd) return;
  try { const r = await invoke('run_command', { command: cmd }); document.getElementById('syslog-output').textContent = r || '(leer)'; document.getElementById('syslog-output').scrollTop = 99999; }
  catch (e) { document.getElementById('syslog-output').textContent = 'Fehler: ' + e; }
}

// ── Ports ──

window.checkPorts = async function checkPorts() {
  const ports = [22, 53, 80, 443, 3000, 3001, 445, 51820, 8080, 8081, 8082, 8087, 8096, 8384, 8989, 7878, 9696, 6767, 9050, 9443, 11434];
  try {
    const results = await invoke('check_ports', { ports });
    let html = '';
    results.forEach(p => {
      html += `<tr><td style="font-family:var(--mono)">${p.port}</td><td>${esc(p.service)}</td><td><span class="badge badge-${p.open ? 'open' : 'closed'}">${p.open ? 'Open' : 'Closed'}</span></td></tr>`;
    });
    document.getElementById('port-table').innerHTML = html;
  } catch (e) {
    document.getElementById('port-table').innerHTML = `<tr><td colspan="3" style="color:var(--red)">Fehler: ${e}</td></tr>`;
  }
}

// ── Power ──

window.confirmPower = function confirmPower(action) {
  const labels = { reboot: 'Server neustarten', shutdown: 'Server herunterfahren' };
  showDialog(labels[action], 'Bist du sicher? Der Server wird ' + (action === 'reboot' ? 'neugestartet' : 'heruntergefahren') + '.', action === 'reboot' ? '🔄' : '⏻', async () => {
    try { await invoke('system_power', { action }); toast(labels[action] + '…'); }
    catch (e) { toast('Fehler: ' + e, 'err'); }
  }, true);
}

// ── Keyboard shortcut ──

document.addEventListener('keydown', e => {
  if (e.ctrlKey && e.key === 'r') {
    e.preventDefault();
    const l = { dashboard: loadDashboard, docker: loadDocker, services: loadServices, files: loadFiles, processes: loadProcesses, storage: loadStorage, ports: checkPorts };
    if (l[currentPage]) l[currentPage]();
    toast('Aktualisiert');
  }
});

// ── Polling ──

function startPolling() {
  if (pollInterval) clearInterval(pollInterval);
  pollInterval = setInterval(() => {
    if (currentPage === 'dashboard' && connected) loadDashboard();
  }, 3000);
}

// ── Init ──

async function init() {
  const savedHost = localStorage.getItem('atlaslab-host');
  const savedUser = localStorage.getItem('atlaslab-user');
  if (savedHost) { document.getElementById('host-input').value = savedHost; if (document.getElementById('m-host')) document.getElementById('m-host').value = savedHost; }
  if (savedUser) { document.getElementById('user-input').value = savedUser; if (document.getElementById('m-user')) document.getElementById('m-user').value = savedUser; }
  await applyConnection();
  if (savedHost) {
    await testConnection();
  } else {
    connected = true;
    document.getElementById('sb-conn-label').textContent = 'Lokal';
    document.getElementById('conn-dot').classList.add('ok');
    await loadDashboard();
  }
  startPolling();
}

init().catch(e => toast('Startfehler: ' + e, 'err'));

// ── CSP Event Delegation ──
document.addEventListener('click', e => {
  const btn = e.target.closest('[data-action]');
  if (!btn) return;
  const action = btn.dataset.action;
  const arg = btn.dataset.arg;

  if (action === 'toggleMore') { toggleMore(); }
  else if (action === 'testConnection') { testConnection(); }
  else if (action === 'loadDocker') { loadDocker(); }
  else if (action === 'loadLogs') { loadLogs(); }
  else if (action === 'loadServices') { loadServices(); }
  else if (action === 'fileMkdir') { fileMkdir(); }
  else if (action === 'loadFiles') { loadFiles(); }
  else if (action === 'closeFileEditor') { closeFileEditor(); }
  else if (action === 'saveFile') { saveFile(); }
  else if (action === 'loadNetwork') { loadNetwork(); }
  else if (action === 'fwAction') { fwAction(arg); }
  else if (action === 'loadFirewall') { loadFirewall(); }
  else if (action === 'loadStorage') { loadStorage(); }
  else if (action === 'loadProcesses') { loadProcesses(); }
  else if (action === 'loadPackages') { loadPackages(); }
  else if (action === 'pkgAction') { pkgAction(arg); }
  else if (action === 'loadUsers') { loadUsers(); }
  else if (action === 'loadCrontab') { loadCrontab(); }
  else if (action === 'saveCrontab') { saveCrontab(); }
  else if (action === 'loadSysLog') { loadSysLog(); }
  else if (action === 'checkPorts') { checkPorts(); }
  else if (action === 'confirmPower') { confirmPower(arg); }
  else if (action === 'closeDialog') { closeDialog(); }
  else if (action === 'navigateFile') { e.preventDefault(); navigateFile(arg); }
  else if (action === 'openFile') { openFile(arg); }
  else if (action === 'deleteFile') { deleteFile(btn.dataset.path, btn.dataset.name, btn.dataset.isdir === 'true'); }
  else if (action === 'killProc') { killProc(arg); }
});

document.addEventListener('input', e => {
  if (e.target.id === 'docker-search') filterDocker();
  else if (e.target.id === 'service-search') filterServices();
  else if (e.target.id === 'proc-search') filterProcs();
});
