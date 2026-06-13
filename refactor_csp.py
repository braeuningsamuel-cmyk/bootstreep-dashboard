import re

def process_index_html():
    with open('src/index.html', 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Simple replacements for onclick
    replacements = [
        ('onclick="toggleMore()"', 'data-action="toggleMore"'),
        ('onclick="testConnection()"', 'data-action="testConnection"'),
        ('onclick="loadDocker()"', 'data-action="loadDocker"'),
        ('onclick="loadLogs()"', 'data-action="loadLogs"'),
        ('onclick="loadServices()"', 'data-action="loadServices"'),
        ('onclick="fileMkdir()"', 'data-action="fileMkdir"'),
        ('onclick="loadFiles()"', 'data-action="loadFiles"'),
        ('onclick="closeFileEditor()"', 'data-action="closeFileEditor"'),
        ('onclick="saveFile()"', 'data-action="saveFile"'),
        ('onclick="loadNetwork()"', 'data-action="loadNetwork"'),
        ('onclick="fwAction(\'allow\')"', 'data-action="fwAction" data-arg="allow"'),
        ('onclick="fwAction(\'deny\')"', 'data-action="fwAction" data-arg="deny"'),
        ('onclick="loadFirewall()"', 'data-action="loadFirewall"'),
        ('onclick="loadStorage()"', 'data-action="loadStorage"'),
        ('onclick="loadProcesses()"', 'data-action="loadProcesses"'),
        ('onclick="loadPackages()"', 'data-action="loadPackages"'),
        ('onclick="pkgAction(\'install\')"', 'data-action="pkgAction" data-arg="install"'),
        ('onclick="pkgAction(\'remove\')"', 'data-action="pkgAction" data-arg="remove"'),
        ('onclick="loadUsers()"', 'data-action="loadUsers"'),
        ('onclick="loadCrontab()"', 'data-action="loadCrontab"'),
        ('onclick="saveCrontab()"', 'data-action="saveCrontab"'),
        ('onclick="loadSysLog()"', 'data-action="loadSysLog"'),
        ('onclick="checkPorts()"', 'data-action="checkPorts"'),
        ('onclick="confirmPower(\'reboot\')"', 'data-action="confirmPower" data-arg="reboot"'),
        ('onclick="confirmPower(\'shutdown\')"', 'data-action="confirmPower" data-arg="shutdown"'),
        ('onclick="closeDialog()"', 'data-action="closeDialog"'),
        ('oninput="filterDocker()"', ''),
        ('oninput="filterServices()"', ''),
        ('oninput="filterProcs()"', ''),
    ]
    
    for old, new in replacements:
        content = content.replace(old, new)
        
    with open('src/index.html', 'w', encoding='utf-8') as f:
        f.write(content)

def process_main_js():
    with open('src/main.js', 'r', encoding='utf-8') as f:
        content = f.read()

    # main.js string HTML generation replacements
    content = content.replace('onclick="navigateFile(\'/\')"', 'data-action="navigateFile" data-arg="/"')
    content = content.replace('onclick="navigateFile(\'${path}\')"', 'data-action="navigateFile" data-arg="${path}"')
    content = content.replace('onclick="navigateFile(\'${parent}\')"', 'data-action="navigateFile" data-arg="${parent}"')
    content = content.replace('onclick="navigateFile(\'${esc(fp)}\');return false"', 'data-action="navigateFile" data-arg="${esc(fp)}"')
    content = content.replace('onclick="openFile(\'${esc(fp)}\')"', 'data-action="openFile" data-arg="${esc(fp)}"')
    content = content.replace('onclick="deleteFile(\'${esc(fp)}\',\'${esc(f.name)}\',${f.is_dir})"', 'data-action="deleteFile" data-path="${esc(fp)}" data-name="${esc(f.name)}" data-isdir="${f.is_dir}"')
    content = content.replace('onclick="killProc(${p.pid})"', 'data-action="killProc" data-arg="${p.pid}"')

    # Add the event delegation listener at the end of the file
    delegation_code = """
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
"""
    if '// ── CSP Event Delegation ──' not in content:
        content += delegation_code

    with open('src/main.js', 'w', encoding='utf-8') as f:
        f.write(content)

process_index_html()
process_main_js()
print("Refactoring completed.")
