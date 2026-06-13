use serde::{Deserialize, Serialize};
use std::io::Write;
use std::net::ToSocketAddrs;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use sysinfo::{Disks, Networks, System};
use tauri::State;

// ─── Data Structures ────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: String,
    pub ports: String,
    pub created: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub mem_percent: f32,
    pub disk_used: u64,
    pub disk_total: u64,
    pub disk_percent: f32,
    pub network_rx: u64,
    pub network_tx: u64,
    pub uptime: u64,
    pub hostname: String,
    pub os: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServiceInfo {
    pub name: String,
    pub status: String,
    pub active: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PortCheck {
    pub port: u16,
    pub open: bool,
    pub service: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionInfo {
    pub host: String,
    pub user: String,
    pub mode: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
    pub permissions: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StorageMount {
    pub filesystem: String,
    pub mount_point: String,
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub percent: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub user: String,
    pub cpu: f32,
    pub mem: f32,
    pub command: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DockerStat {
    pub name: String,
    pub cpu_percent: String,
    pub mem_usage: String,
    pub mem_limit: String,
    pub net_io: String,
    pub block_io: String,
    pub pids: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserEntry {
    pub name: String,
    pub uid: u32,
    pub gid: u32,
    pub home: String,
    pub shell: String,
    pub info: String,
}

// ─── App State ──────────────────────────────────────────────────────────────

pub struct AppConfig {
    remote_host: Mutex<String>,
    remote_user: Mutex<String>,
    sys: Mutex<System>,
    networks: Mutex<Networks>,
    disks: Mutex<Disks>,
}

// ─── Helper Functions ───────────────────────────────────────────────────────

fn run_cmd(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", cmd, e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() && !stderr.is_empty() {
        Err(stderr.trim().to_string())
    } else {
        Ok(stdout.trim().to_string())
    }
}

fn ssh_target(host: &str, user: &str) -> String {
    if user.is_empty() {
        host.to_string()
    } else {
        format!("{}@{}", user, host)
    }
}

fn shell_escape(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/')
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace("'", "'\\''"))
    }
}

fn is_remote(config: &AppConfig) -> bool {
    !config.remote_host.lock().unwrap().is_empty()
}

fn run_ssh(config: &AppConfig, remote_cmd: &str) -> Result<String, String> {
    let host = config.remote_host.lock().unwrap().clone();
    let user = config.remote_user.lock().unwrap().clone();
    let target = ssh_target(&host, &user);
    run_cmd(
        "ssh",
        &[
            "-o", "BatchMode=yes",
            "-o", "ConnectTimeout=8",
            "-o", "StrictHostKeyChecking=accept-new",
            &target,
            remote_cmd,
        ],
    )
}

fn ssh_write_stdin(config: &AppConfig, remote_cmd: &str, input: &str) -> Result<String, String> {
    let host = config.remote_host.lock().unwrap().clone();
    let user = config.remote_user.lock().unwrap().clone();
    let target = ssh_target(&host, &user);

    let mut child = Command::new("ssh")
        .args(&[
            "-o", "BatchMode=yes",
            "-o", "ConnectTimeout=8",
            "-o", "StrictHostKeyChecking=accept-new",
            &target,
            remote_cmd,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn ssh: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Wait error: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() || stderr.is_empty() {
        Ok(stdout.trim().to_string())
    } else {
        Err(stderr.trim().to_string())
    }
}

fn run_on_target(
    config: &AppConfig,
    local_cmd: &str,
    local_args: &[&str],
    remote_cmd: &str,
) -> Result<String, String> {
    if is_remote(config) {
        run_ssh(config, remote_cmd)
    } else {
        run_cmd(local_cmd, local_args)
    }
}

fn local_shell_cmd(cmd: &str) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        run_cmd("powershell", &["-Command", cmd])
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_cmd("sh", &["-c", cmd])
    }
}

// ─── System Stats ───────────────────────────────────────────────────────────

fn local_system_stats(config: &AppConfig) -> Result<SystemStats, String> {
    let mut sys = config.sys.lock().unwrap();
    sys.refresh_all();

    let cpu_usage = sys.global_cpu_usage();
    let mem_used = sys.used_memory();
    let mem_total = sys.total_memory();
    let mem_percent = if mem_total > 0 {
        (mem_used as f32 / mem_total as f32) * 100.0
    } else {
        0.0
    };

    let mut disks = config.disks.lock().unwrap();
    disks.refresh(true);
    let (disk_used, disk_total, disk_percent) = if let Some(disk) = disks.first() {
        let disk_total = disk.total_space();
        let disk_used = disk_total - disk.available_space();
        let disk_percent = if disk_total > 0 {
            (disk_used as f32 / disk_total as f32) * 100.0
        } else {
            0.0
        };
        (disk_used, disk_total, disk_percent)
    } else {
        (0, 0, 0.0)
    };

    let mut networks = config.networks.lock().unwrap();
    networks.refresh(true);
    let mut network_rx: u64 = 0;
    let mut network_tx: u64 = 0;
    for (_name, data) in networks.iter() {
        network_rx += data.received();
        network_tx += data.transmitted();
    }

    let hostname = System::host_name().unwrap_or_default();
    let os = System::long_os_version().unwrap_or_default();

    Ok(SystemStats {
        cpu_usage,
        mem_used,
        mem_total,
        mem_percent,
        disk_used,
        disk_total,
        disk_percent,
        network_rx,
        network_tx,
        uptime: System::uptime(),
        hostname,
        os,
    })
}

fn remote_system_stats(config: &AppConfig) -> Result<SystemStats, String> {
    const SCRIPT: &str = r#"
export LANG=C
MT=$(awk '/MemTotal:/{print $2}' /proc/meminfo)
MA=$(awk '/MemAvailable:/{print $2}' /proc/meminfo)
MU=$((MT-MA))
DT=$(df -B1 / 2>/dev/null | tail -1 | awk '{print $2}')
DU=$(df -B1 / 2>/dev/null | tail -1 | awk '{print $3}')
UP=$(cut -d. -f1 /proc/uptime)
RX=$(awk 'NR>2{rx+=$2}END{print rx+0}' /proc/net/dev)
TX=$(awk 'NR>2{tx+=$10}END{print tx+0}' /proc/net/dev)
read _ u n s id wa ir sir steal < /proc/stat
TOT=$((u+n+s+id+wa+ir+sir+steal))
CPU=$(awk -v u=$u -v n=$n -v s=$s -v t=$TOT 'BEGIN{if(t>0) printf "%.1f", (u+n+s)*100/t; else print "0"}')
echo "${CPU}|${MU}|${MT}|${DU}|${DT}|${RX}|${TX}|${UP}|$(hostname)|$(uname -sr)"
"#;

    let output = run_ssh(config, SCRIPT)?;
    let line = output.lines().last().unwrap_or(&output);
    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() < 10 {
        return Err(format!("Unexpected stats format: {}", output));
    }

    let cpu_usage = parts[0].parse::<f32>().unwrap_or(0.0);
    let mem_used_kb = parts[1].parse::<u64>().unwrap_or(0);
    let mem_total_kb = parts[2].parse::<u64>().unwrap_or(0);
    let mem_used = mem_used_kb * 1024;
    let mem_total = mem_total_kb * 1024;
    let mem_percent = if mem_total > 0 {
        (mem_used as f32 / mem_total as f32) * 100.0
    } else {
        0.0
    };

    let disk_used = parts[3].parse::<u64>().unwrap_or(0);
    let disk_total = parts[4].parse::<u64>().unwrap_or(0);
    let disk_percent = if disk_total > 0 {
        (disk_used as f32 / disk_total as f32) * 100.0
    } else {
        0.0
    };

    let network_rx = parts[5].parse::<u64>().unwrap_or(0);
    let network_tx = parts[6].parse::<u64>().unwrap_or(0);
    let uptime = parts[7].parse::<u64>().unwrap_or(0);

    Ok(SystemStats {
        cpu_usage,
        mem_used,
        mem_total,
        mem_percent,
        disk_used,
        disk_total,
        disk_percent,
        network_rx,
        network_tx,
        uptime,
        hostname: parts[8].to_string(),
        os: parts[9].to_string(),
    })
}

// ─── Connection Commands ────────────────────────────────────────────────────

#[tauri::command]
fn set_connection(
    host: String,
    user: String,
    config: State<'_, AppConfig>,
) -> Result<ConnectionInfo, String> {
    *config.remote_host.lock().unwrap() = host.trim().to_string();
    *config.remote_user.lock().unwrap() = user.trim().to_string();
    get_connection(config)
}

#[tauri::command]
fn get_connection(config: State<'_, AppConfig>) -> Result<ConnectionInfo, String> {
    let host = config.remote_host.lock().unwrap().clone();
    let user = config.remote_user.lock().unwrap().clone();
    let mode = if host.is_empty() {
        "local".to_string()
    } else {
        "remote".to_string()
    };
    Ok(ConnectionInfo { host, user, mode })
}

#[tauri::command]
fn test_ssh_connection(config: State<'_, AppConfig>) -> Result<bool, String> {
    if is_remote(&config) {
        let ok = run_ssh(&config, "echo ok").map(|o| o.contains("ok"))?;
        Ok(ok)
    } else {
        local_system_stats(&config)?;
        Ok(true)
    }
}

// ─── System Stats Command ───────────────────────────────────────────────────

#[tauri::command]
fn system_stats(config: State<'_, AppConfig>) -> Result<SystemStats, String> {
    if is_remote(&config) {
        remote_system_stats(&config)
    } else {
        local_system_stats(&config)
    }
}

#[tauri::command]
fn get_uptime(config: State<'_, AppConfig>) -> Result<u64, String> {
    if is_remote(&config) {
        let output = run_ssh(&config, "cut -d. -f1 /proc/uptime")?;
        output
            .trim()
            .parse::<u64>()
            .map_err(|e| format!("Invalid uptime: {}", e))
    } else {
        Ok(System::uptime())
    }
}

// ─── Docker Commands ────────────────────────────────────────────────────────

#[tauri::command]
fn docker_list(config: State<'_, AppConfig>) -> Result<Vec<ContainerInfo>, String> {
    let format_str =
        "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.State}}\t{{.Ports}}\t{{.CreatedAt}}";
    let remote_cmd = format!(
        "docker ps -a --format '{}'",
        format_str.replace("'", "'\\''")
    );
    let output = run_on_target(
        &config,
        "docker",
        &["ps", "-a", "--format", format_str],
        &remote_cmd,
    )?;

    let mut containers = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 7 {
            containers.push(ContainerInfo {
                id: parts[0].to_string(),
                name: parts[1].to_string(),
                image: parts[2].to_string(),
                status: parts[3].to_string(),
                state: parts[4].to_string(),
                ports: parts[5].to_string(),
                created: parts[6].to_string(),
            });
        }
    }
    Ok(containers)
}

#[tauri::command]
fn docker_action(
    action: String,
    name: String,
    config: State<'_, AppConfig>,
) -> Result<String, String> {
    let valid = ["start", "stop", "restart", "remove", "pause", "unpause"];
    if !valid.contains(&action.as_str()) {
        return Err("Invalid action".to_string());
    }
    let escaped = shell_escape(&name);
    let remote_cmd = format!("docker {} {}", action, escaped);
    run_on_target(&config, "docker", &[&action, &name], &remote_cmd)
}

#[tauri::command]
fn docker_logs(
    name: String,
    lines: Option<u32>,
    config: State<'_, AppConfig>,
) -> Result<String, String> {
    let n = lines.unwrap_or(100);
    let escaped = shell_escape(&name);
    let remote_cmd = format!("docker logs --tail {} {} 2>&1", n, escaped);
    run_on_target(
        &config,
        "docker",
        &["logs", "--tail", &n.to_string(), &name],
        &remote_cmd,
    )
}

#[tauri::command]
fn docker_exec(
    name: String,
    command: String,
    config: State<'_, AppConfig>,
) -> Result<String, String> {
    let escaped_name = shell_escape(&name);
    let escaped_cmd = shell_escape(&command);
    let remote_cmd = format!("docker exec {} sh -c {}", escaped_name, escaped_cmd);
    run_on_target(
        &config,
        "docker",
        &["exec", &name, "sh", "-c", &command],
        &remote_cmd,
    )
}

#[tauri::command]
fn docker_stats(config: State<'_, AppConfig>) -> Result<Vec<DockerStat>, String> {
    let format_str = "{{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.MemPerc}}\t{{.NetIO}}\t{{.BlockIO}}\t{{.PIDs}}";
    let remote_cmd = format!(
        "docker stats --no-stream --format '{}'",
        format_str.replace("'", "'\\''")
    );
    let output = run_on_target(
        &config,
        "docker",
        &["stats", "--no-stream", "--format", format_str],
        &remote_cmd,
    )?;

    let mut stats = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 7 {
            stats.push(DockerStat {
                name: parts[0].to_string(),
                cpu_percent: parts[1].to_string(),
                mem_usage: parts[2].to_string(),
                mem_limit: parts[3].to_string(),
                net_io: parts[4].to_string(),
                block_io: parts[5].to_string(),
                pids: parts[6].to_string(),
            });
        }
    }
    Ok(stats)
}

// ─── Service Commands ───────────────────────────────────────────────────────

#[tauri::command]
fn service_list(config: State<'_, AppConfig>) -> Result<Vec<ServiceInfo>, String> {
    let remote_cmd =
        "systemctl list-units --type=service --all --no-pager --no-legend --plain --no-ansi";
    let output = run_on_target(
        &config,
        "systemctl",
        &[
            "list-units",
            "--type=service",
            "--all",
            "--no-pager",
            "--no-legend",
            "--plain",
            "--no-ansi",
        ],
        remote_cmd,
    )?;

    let mut services = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let name = parts[0]
                .strip_suffix(".service")
                .unwrap_or(parts[0])
                .to_string();
            let active = parts[1].to_string();
            let status = parts[2].to_string();
            let description: Vec<&str> = parts[3..].iter().copied().collect();
            services.push(ServiceInfo {
                name,
                status,
                active,
                description: description.join(" "),
            });
        }
    }
    Ok(services)
}

#[tauri::command]
fn service_action(
    action: String,
    name: String,
    config: State<'_, AppConfig>,
) -> Result<String, String> {
    let valid = ["start", "stop", "restart", "enable", "disable"];
    if !valid.contains(&action.as_str()) {
        return Err("Invalid action".to_string());
    }
    let escaped = shell_escape(&name);
    let remote_cmd = format!("sudo systemctl {} {}", action, escaped);
    if is_remote(&config) {
        run_ssh(&config, &remote_cmd)
    } else {
        run_cmd("systemctl", &[&action, &name])
    }
}

// ─── Port Check (Cross-Platform) ───────────────────────────────────────────

#[tauri::command]
fn check_ports(ports: Vec<u16>, config: State<'_, AppConfig>) -> Result<Vec<PortCheck>, String> {
    let host = config.remote_host.lock().unwrap().clone();
    let target = if host.is_empty() {
        "127.0.0.1".to_string()
    } else {
        host
    };

    let services_map: std::collections::HashMap<u16, &str> = [
        (22, "SSH"),
        (53, "Pi-hole DNS"),
        (80, "HTTP"),
        (443, "HTTPS"),
        (3000, "Hermes"),
        (3001, "Uptime Kuma"),
        (445, "Samba"),
        (51820, "WireGuard"),
        (8080, "Websurfx"),
        (8081, "Pi-hole"),
        (8082, "Nextcloud"),
        (8087, "AMP"),
        (8096, "Jellyfin"),
        (8384, "Syncthing"),
        (8989, "Sonarr"),
        (7878, "Radarr"),
        (9696, "Prowlarr"),
        (6767, "Bazarr"),
        (9050, "Tor"),
        (9443, "Nextcloud SSL"),
        (11434, "Ollama"),
    ]
    .iter()
    .cloned()
    .collect();

    let mut results = Vec::new();
    for port in ports {
        let addr_str = format!("{}:{}", target, port);
        let open = if let Ok(mut addrs) = addr_str.to_socket_addrs() {
            if let Some(addr) = addrs.next() {
                std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok()
            } else {
                false
            }
        } else {
            false
        };
        let service = services_map
            .get(&port)
            .unwrap_or(&"Unknown")
            .to_string();
        results.push(PortCheck {
            port,
            open,
            service,
        });
    }
    Ok(results)
}

// ─── Terminal / Run Command ─────────────────────────────────────────────────

#[tauri::command]
fn run_command(command: String, config: State<'_, AppConfig>) -> Result<String, String> {
    if is_remote(&config) {
        let escaped = shell_escape(&command);
        run_ssh(&config, &format!("bash -lc {}", escaped))
    } else {
        local_shell_cmd(&command)
    }
}

// ─── File Operations ────────────────────────────────────────────────────────

#[tauri::command]
fn file_list(path: String, config: State<'_, AppConfig>) -> Result<Vec<FileEntry>, String> {
    let escaped = shell_escape(&path);

    if is_remote(&config) {
        // Use stat-based listing for reliable parsing
        let script = format!(
            r#"for f in {}/*; do [ -e "$f" ] || continue; stat --format='%n|%F|%s|%y|%A' -- "$f" 2>/dev/null; done; for f in {}/.*; do b=$(basename "$f"); [ "$b" = "." ] || [ "$b" = ".." ] && continue; [ -e "$f" ] || continue; stat --format='%n|%F|%s|%y|%A' -- "$f" 2>/dev/null; done"#,
            escaped, escaped
        );
        let output = run_ssh(&config, &script)?;
        parse_file_entries(&output)
    } else {
        let p = std::path::Path::new(&path);
        if !p.exists() {
            return Err(format!("Path does not exist: {}", path));
        }
        let mut entries = Vec::new();
        let rd =
            std::fs::read_dir(p).map_err(|e| format!("Cannot read directory {}: {}", path, e))?;
        for entry in rd {
            if let Ok(entry) = entry {
                let metadata = entry.metadata().unwrap_or_else(|_| {
                    std::fs::metadata(entry.path()).expect("cannot get metadata")
                });
                let modified = metadata
                    .modified()
                    .map(|t| {
                        let dur = t
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default();
                        format!("{}", dur.as_secs())
                    })
                    .unwrap_or_default();
                entries.push(FileEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    is_dir: metadata.is_dir(),
                    size: metadata.len(),
                    modified,
                    permissions: if metadata.is_dir() {
                        "drwxr-xr-x".to_string()
                    } else {
                        "-rw-r--r--".to_string()
                    },
                });
            }
        }
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(entries)
    }
}

fn parse_file_entries(output: &str) -> Result<Vec<FileEntry>, String> {
    let mut entries = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(5, '|').collect();
        if parts.len() >= 5 {
            let full_path = parts[0];
            let name = full_path
                .rsplit('/')
                .next()
                .unwrap_or(full_path)
                .to_string();
            let file_type = parts[1];
            let is_dir = file_type.contains("directory");
            let size = parts[2].parse::<u64>().unwrap_or(0);
            let modified = parts[3].to_string();
            let permissions = parts[4].to_string();
            entries.push(FileEntry {
                name,
                is_dir,
                size,
                modified,
                permissions,
            });
        }
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

#[tauri::command]
fn file_read(path: String, config: State<'_, AppConfig>) -> Result<String, String> {
    if is_remote(&config) {
        let escaped = shell_escape(&path);
        run_ssh(&config, &format!("cat -- {}", escaped))
    } else {
        std::fs::read_to_string(&path).map_err(|e| format!("Read error: {}", e))
    }
}

#[tauri::command]
fn file_write(path: String, content: String, config: State<'_, AppConfig>) -> Result<String, String> {
    if is_remote(&config) {
        let escaped = shell_escape(&path);
        // Prefix with ./ if it starts with - and path is a relative, but here path is supposed to be absolute or relative to home. For shell redirect `>` `-` is fine.
        let remote_cmd = format!("cat > {}", escaped);
        ssh_write_stdin(&config, &remote_cmd, &content)
    } else {
        std::fs::write(&path, &content).map_err(|e| format!("Write error: {}", e))?;
        Ok("OK".to_string())
    }
}

#[tauri::command]
fn file_delete(path: String, config: State<'_, AppConfig>) -> Result<String, String> {
    let escaped = shell_escape(&path);
    if is_remote(&config) {
        run_ssh(&config, &format!("rm -rf -- {}", escaped))
    } else {
        let p = std::path::Path::new(&path);
        if p.is_dir() {
            std::fs::remove_dir_all(p).map_err(|e| format!("Delete error: {}", e))?;
        } else {
            std::fs::remove_file(p).map_err(|e| format!("Delete error: {}", e))?;
        }
        Ok("OK".to_string())
    }
}

#[tauri::command]
fn file_mkdir(path: String, config: State<'_, AppConfig>) -> Result<String, String> {
    let escaped = shell_escape(&path);
    if is_remote(&config) {
        run_ssh(&config, &format!("mkdir -p -- {}", escaped))
    } else {
        std::fs::create_dir_all(&path).map_err(|e| format!("Mkdir error: {}", e))?;
        Ok("OK".to_string())
    }
}

#[tauri::command]
fn file_rename(from: String, to: String, config: State<'_, AppConfig>) -> Result<String, String> {
    let esc_from = shell_escape(&from);
    let esc_to = shell_escape(&to);
    if is_remote(&config) {
        run_ssh(&config, &format!("mv -- {} {}", esc_from, esc_to))
    } else {
        std::fs::rename(&from, &to).map_err(|e| format!("Rename error: {}", e))?;
        Ok("OK".to_string())
    }
}

// ─── Network Commands ───────────────────────────────────────────────────────

#[tauri::command]
fn network_info(config: State<'_, AppConfig>) -> Result<String, String> {
    let script = r#"echo "=== INTERFACES ==="; ip -br addr show 2>/dev/null || ifconfig 2>/dev/null; echo "=== ROUTES ==="; ip route show 2>/dev/null || route -n 2>/dev/null; echo "=== DNS ==="; cat /etc/resolv.conf 2>/dev/null | grep -v '^#'"#;
    if is_remote(&config) {
        run_ssh(&config, script)
    } else {
        local_shell_cmd(script)
    }
}

#[tauri::command]
fn firewall_status(config: State<'_, AppConfig>) -> Result<String, String> {
    let script = r#"sudo ufw status numbered 2>/dev/null || sudo iptables -L -n --line-numbers 2>/dev/null || echo "No firewall found""#;
    if is_remote(&config) {
        run_ssh(&config, script)
    } else {
        local_shell_cmd(script)
    }
}

#[tauri::command]
fn firewall_action(
    action: String,
    rule: String,
    config: State<'_, AppConfig>,
) -> Result<String, String> {
    let valid = ["allow", "deny", "delete", "enable", "disable", "reload"];
    if !valid.contains(&action.as_str()) {
        return Err("Invalid firewall action".to_string());
    }
    if is_remote(&config) {
        let cmd = format!("sudo ufw {} {}", action, shell_escape(&rule));
        run_ssh(&config, &cmd)
    } else {
        run_cmd("sudo", &["ufw", &action, &rule])
    }
}

// ─── Storage Commands ───────────────────────────────────────────────────────

#[tauri::command]
fn storage_info(config: State<'_, AppConfig>) -> Result<Vec<StorageMount>, String> {
    if is_remote(&config) {
        let output = run_ssh(
            &config,
            "df -B1 --output=source,target,size,used,avail,pcent -x tmpfs -x devtmpfs -x squashfs 2>/dev/null | tail -n +2",
        )?;
        let mut mounts = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                let percent_str = parts[5].trim_end_matches('%');
                mounts.push(StorageMount {
                    filesystem: parts[0].to_string(),
                    mount_point: parts[1].to_string(),
                    total: parts[2].parse().unwrap_or(0),
                    used: parts[3].parse().unwrap_or(0),
                    available: parts[4].parse().unwrap_or(0),
                    percent: percent_str.parse().unwrap_or(0.0),
                });
            }
        }
        Ok(mounts)
    } else {
        let mut disks = config.disks.lock().unwrap();
        disks.refresh(true);
        let mut mounts = Vec::new();
        for disk in disks.iter() {
            let total = disk.total_space();
            let avail = disk.available_space();
            let used = total - avail;
            let percent = if total > 0 {
                (used as f32 / total as f32) * 100.0
            } else {
                0.0
            };
            mounts.push(StorageMount {
                filesystem: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total,
                used,
                available: avail,
                percent,
            });
        }
        Ok(mounts)
    }
}

// ─── Process Commands ───────────────────────────────────────────────────────

#[tauri::command]
fn process_list(config: State<'_, AppConfig>) -> Result<Vec<ProcessInfo>, String> {
    if is_remote(&config) {
        let output = run_ssh(
            &config,
            "ps aux --sort=-%cpu --no-headers 2>/dev/null | head -100",
        )?;
        let mut procs = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 11 {
                procs.push(ProcessInfo {
                    pid: parts[1].parse().unwrap_or(0),
                    user: parts[0].to_string(),
                    cpu: parts[2].parse().unwrap_or(0.0),
                    mem: parts[3].parse().unwrap_or(0.0),
                    command: parts[10..].join(" "),
                });
            }
        }
        Ok(procs)
    } else {
        let mut sys = config.sys.lock().unwrap();
        sys.refresh_all();
        let mut procs: Vec<ProcessInfo> = sys
            .processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                pid: pid.as_u32(),
                user: String::new(),
                cpu: process.cpu_usage(),
                mem: process.memory() as f32 / (1024.0 * 1024.0),
                command: process.name().to_string_lossy().to_string(),
            })
            .collect();
        procs.sort_by(|a, b| {
            b.cpu
                .partial_cmp(&a.cpu)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        procs.truncate(100);
        Ok(procs)
    }
}

#[tauri::command]
fn process_kill(
    pid: u32,
    signal: Option<String>,
    config: State<'_, AppConfig>,
) -> Result<String, String> {
    let sig = signal.unwrap_or_else(|| "TERM".to_string());
    let cmd = format!("kill -{} {}", sig, pid);
    if is_remote(&config) {
        run_ssh(&config, &cmd)
    } else {
        local_shell_cmd(&cmd)
    }
}

// ─── Crontab Commands ───────────────────────────────────────────────────────

#[tauri::command]
fn crontab_list(config: State<'_, AppConfig>) -> Result<String, String> {
    if is_remote(&config) {
        run_ssh(&config, "crontab -l 2>/dev/null || echo '(keine Cronjobs)'")
    } else {
        local_shell_cmd("crontab -l 2>/dev/null || echo '(keine Cronjobs)'")
    }
}

#[tauri::command]
fn crontab_save(content: String, config: State<'_, AppConfig>) -> Result<String, String> {
    if is_remote(&config) {
        ssh_write_stdin(&config, "crontab -", &content)
    } else {
        // Write to temp file and load
        let tmp = std::env::temp_dir().join("atlaslab_crontab.tmp");
        std::fs::write(&tmp, &content).map_err(|e| format!("Write error: {}", e))?;
        let result = run_cmd(
            "crontab",
            &[tmp.to_str().unwrap_or("/tmp/atlaslab_crontab.tmp")],
        );
        let _ = std::fs::remove_file(&tmp);
        result
    }
}

// ─── Package Commands ───────────────────────────────────────────────────────

#[tauri::command]
fn package_updates(config: State<'_, AppConfig>) -> Result<String, String> {
    let script = r#"if command -v apt >/dev/null 2>&1; then apt list --upgradable 2>/dev/null; elif command -v dnf >/dev/null 2>&1; then dnf check-update 2>/dev/null; elif command -v pacman >/dev/null 2>&1; then pacman -Qu 2>/dev/null; else echo "Kein unterstützter Paketmanager gefunden"; fi"#;
    if is_remote(&config) {
        run_ssh(&config, script)
    } else {
        local_shell_cmd(script)
    }
}

#[tauri::command]
fn package_action(
    name: String,
    action: String,
    config: State<'_, AppConfig>,
) -> Result<String, String> {
    let valid = ["install", "remove", "update"];
    if !valid.contains(&action.as_str()) {
        return Err("Invalid package action".to_string());
    }
    let escaped = shell_escape(&name);
    let script = match action.as_str() {
        "install" => format!(
            r#"if command -v apt >/dev/null 2>&1; then sudo apt install -y {}; elif command -v dnf >/dev/null 2>&1; then sudo dnf install -y {}; elif command -v pacman >/dev/null 2>&1; then sudo pacman -S --noconfirm {}; else echo "Kein Paketmanager"; fi"#,
            escaped, escaped, escaped
        ),
        "remove" => format!(
            r#"if command -v apt >/dev/null 2>&1; then sudo apt remove -y {}; elif command -v dnf >/dev/null 2>&1; then sudo dnf remove -y {}; elif command -v pacman >/dev/null 2>&1; then sudo pacman -R --noconfirm {}; else echo "Kein Paketmanager"; fi"#,
            escaped, escaped, escaped
        ),
        "update" => r#"if command -v apt >/dev/null 2>&1; then sudo apt update && sudo apt upgrade -y; elif command -v dnf >/dev/null 2>&1; then sudo dnf upgrade -y; elif command -v pacman >/dev/null 2>&1; then sudo pacman -Syu --noconfirm; else echo "Kein Paketmanager"; fi"#.to_string(),
        _ => return Err("Invalid package action".to_string()),
    };
    if is_remote(&config) {
        run_ssh(&config, &script)
    } else {
        local_shell_cmd(&script)
    }
}

// ─── User Commands ──────────────────────────────────────────────────────────

#[tauri::command]
fn user_list(config: State<'_, AppConfig>) -> Result<Vec<UserEntry>, String> {
    let script = "cat /etc/passwd";
    let output = if is_remote(&config) {
        run_ssh(&config, script)?
    } else {
        local_shell_cmd(script)?
    };

    let mut users = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 7 {
            let uid = parts[2].parse::<u32>().unwrap_or(0);
            // Only show real users (uid >= 1000) and root
            if uid >= 1000 || uid == 0 {
                users.push(UserEntry {
                    name: parts[0].to_string(),
                    uid,
                    gid: parts[3].parse().unwrap_or(0),
                    home: parts[5].to_string(),
                    shell: parts[6].to_string(),
                    info: parts[4].to_string(),
                });
            }
        }
    }
    Ok(users)
}

// ─── System Power Commands ──────────────────────────────────────────────────

#[tauri::command]
fn system_power(action: String, config: State<'_, AppConfig>) -> Result<String, String> {
    let valid = ["reboot", "shutdown", "poweroff"];
    if !valid.contains(&action.as_str()) {
        return Err("Invalid power action".to_string());
    }
    let cmd = match action.as_str() {
        "reboot" => "sudo reboot",
        "shutdown" | "poweroff" => "sudo shutdown -h now",
        _ => return Err("Invalid action".to_string()),
    };
    if is_remote(&config) {
        // Run async - won't get response since server is shutting down
        let _ = run_ssh(&config, cmd);
        Ok(format!("{} initiated", action))
    } else {
        local_shell_cmd(cmd)
    }
}

// ─── App Entry Point ────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppConfig {
            remote_host: Mutex::new(String::new()),
            remote_user: Mutex::new(String::new()),
            sys: Mutex::new(System::new_all()),
            networks: Mutex::new(Networks::new_with_refreshed_list()),
            disks: Mutex::new(Disks::new_with_refreshed_list()),
        })
        .invoke_handler(tauri::generate_handler![
            // Connection
            set_connection,
            get_connection,
            test_ssh_connection,
            // System
            system_stats,
            get_uptime,
            system_power,
            // Docker
            docker_list,
            docker_action,
            docker_logs,
            docker_exec,
            docker_stats,
            // Services
            service_list,
            service_action,
            // Ports
            check_ports,
            // Terminal
            run_command,
            // Files
            file_list,
            file_read,
            file_write,
            file_delete,
            file_mkdir,
            file_rename,
            // Network
            network_info,
            firewall_status,
            firewall_action,
            // Storage
            storage_info,
            // Processes
            process_list,
            process_kill,
            // Crontab
            crontab_list,
            crontab_save,
            // Packages
            package_updates,
            package_action,
            // Users
            user_list,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
