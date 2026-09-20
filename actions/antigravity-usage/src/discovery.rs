// specs/005_antigravity_usage.md
use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub port: u16,
    pub csrf_token: String,
}

/// Discovers the active Antigravity Language Server on Linux by scanning /proc,
/// extracting the CSRF token and listening ports, and verifying responsiveness.
pub async fn discover_connection() -> Option<ConnectionInfo> {
    let candidates = find_antigravity_candidates()?;
    for (pid, csrf_token) in candidates {
        let ports = find_listening_ports(pid);
        for port in ports {
            if verify_port(port, &csrf_token).await {
                return Some(ConnectionInfo { port, csrf_token });
            }
        }
    }
    None
}

/// Verifies whether the specified port and CSRF token respond to the Antigravity Language Server API.
pub async fn verify_port(port: u16, csrf_token: &str) -> bool {
    let client = match reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_millis(1500))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    let url = format!(
        "https://127.0.0.1:{port}/exa.language_server_pb.LanguageServerService/GetUserStatus"
    );

    let res = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Connect-Protocol-Version", "1")
        .header("X-Codeium-Csrf-Token", csrf_token)
        .json(&serde_json::json!({
            "metadata": {
                "ideName": "antigravity",
                "extensionName": "antigravity",
                "locale": "en"
            }
        }))
        .send()
        .await;

    match res {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// Scans /proc to find (PID, CSRF token) pairs for running Antigravity Language Server processes.
pub fn find_antigravity_candidates() -> Option<Vec<(u32, String)>> {
    let mut results = Vec::new();
    let proc_dir = fs::read_dir("/proc").ok()?;

    for entry in proc_dir.flatten() {
        let file_name = entry.file_name();
        let name_str = file_name.to_str()?;
        let pid: u32 = match name_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let cmdline_path = entry.path().join("cmdline");
        if let Ok(bytes) = fs::read(cmdline_path) {
            let cmdline = String::from_utf8_lossy(&bytes);
            if is_antigravity_language_server(&cmdline) {
                if let Some(token) = extract_csrf_token(&cmdline) {
                    results.push((pid, token));
                }
            }
        }
    }

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

/// Determines if command line belongs to an Antigravity language_server process.
pub fn is_antigravity_language_server(cmdline: &str) -> bool {
    let lower = cmdline.to_lowercase();
    let has_ls = lower.contains("language_server");
    let has_ag = lower.contains("antigravity");
    has_ls && has_ag
}

/// Extracts the CSRF token from process command line string.
pub fn extract_csrf_token(cmdline: &str) -> Option<String> {
    // Arguments in /proc/<pid>/cmdline are separated by '\0'
    let args: Vec<&str> = cmdline.split('\0').collect();
    for (i, &arg) in args.iter().enumerate() {
        if arg == "--csrf_token" {
            if let Some(&val) = args.get(i + 1) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        } else if let Some(val) = arg.strip_prefix("--csrf_token=") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    // Fallback for space-separated arguments if raw string was passed
    let tokens: Vec<&str> = cmdline.split_whitespace().collect();
    for (i, &tok) in tokens.iter().enumerate() {
        if tok == "--csrf_token" {
            if let Some(&val) = tokens.get(i + 1) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        } else if let Some(val) = tok.strip_prefix("--csrf_token=") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

/// Finds listening ports for a given process PID by inspecting /proc/<pid>/fd and /proc/net/tcp.
pub fn find_listening_ports(pid: u32) -> Vec<u16> {
    let mut ports = Vec::new();
    let fd_dir = PathBuf::from(format!("/proc/{pid}/fd"));
    let mut inodes: HashSet<String> = HashSet::new();

    if let Ok(entries) = fs::read_dir(fd_dir) {
        for entry in entries.flatten() {
            if let Ok(target) = fs::read_link(entry.path()) {
                let target_str = target.to_string_lossy();
                if target_str.starts_with("socket:[") && target_str.ends_with(']') {
                    let inode = &target_str[8..target_str.len() - 1];
                    inodes.insert(inode.to_string());
                }
            }
        }
    }

    // Parse /proc/net/tcp and /proc/net/tcp6
    for path in ["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(content) = fs::read_to_string(path) {
            ports.extend(parse_proc_net_tcp(&content, &inodes));
        }
    }

    // Fallback using ss if inodes check yielded nothing
    if ports.is_empty() {
        if let Ok(output) = std::process::Command::new("ss")
            .args(["-tlnp"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                ports.extend(parse_ss_output(&stdout, pid));
            }
        }
    }

    ports.sort_unstable();
    ports.dedup();
    ports
}

/// Parses /proc/net/tcp format to extract listening ports matching given socket inodes.
pub fn parse_proc_net_tcp(content: &str, target_inodes: &HashSet<String>) -> Vec<u16> {
    let mut ports = Vec::new();
    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }

        // State "0A" indicates TCP_LISTEN
        let state = parts[3];
        if state != "0A" {
            continue;
        }

        let inode = parts[9];
        if !target_inodes.is_empty() && !target_inodes.contains(inode) {
            continue;
        }

        // Local address is in format "0100007F:A2F7" (IP:Port in hex)
        let local_addr = parts[1];
        if let Some((_, port_hex)) = local_addr.split_once(':') {
            if let Ok(port) = u16::from_str_radix(port_hex, 16) {
                ports.push(port);
            }
        }
    }
    ports
}

/// Parses `ss -tlnp` output for listening ports associated with a PID.
pub fn parse_ss_output(stdout: &str, pid: u32) -> Vec<u16> {
    let mut ports = Vec::new();
    let pid_marker = format!("pid={pid},");

    for line in stdout.lines() {
        if !line.contains("LISTEN") || !line.contains(&pid_marker) {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        // Column 4 is usually Local Address:Port (e.g. 127.0.0.1:41719)
        for part in &parts {
            if let Some(pos) = part.rfind(':') {
                if let Ok(port) = part[pos + 1..].parse::<u16>() {
                    ports.push(port);
                    break;
                }
            }
        }
    }
    ports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_csrf_token_from_null_delimited_cmdline() {
        let cmd = "language_server\0--override_ide_name\0antigravity\0--csrf_token\0abc-123-xyz\0--app_data_dir\0antigravity";
        assert_eq!(extract_csrf_token(cmd), Some("abc-123-xyz".to_string()));
    }

    #[test]
    fn extracts_csrf_token_with_equals_sign() {
        let cmd = "language_server --csrf_token=secret-token-456 --app_data_dir antigravity";
        assert_eq!(
            extract_csrf_token(cmd),
            Some("secret-token-456".to_string())
        );
    }

    #[test]
    fn identifies_antigravity_language_server_process() {
        let cmd1 = "/opt/Antigravity/resources/bin/language_server --override_ide_name antigravity";
        assert!(is_antigravity_language_server(cmd1));

        let cmd2 = "python3 some_script.py";
        assert!(!is_antigravity_language_server(cmd2));
    }

    #[test]
    fn parses_proc_net_tcp_listening_sockets() {
        let sample = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n   9: 0100007F:A2F7 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 739149 2 0000000000000000 100 0 0 10 0\n  10: 0100007F:1F90 00000000:0000 01 00000000:00000000 00:00000000 00000000  1000        0 999999 2 0000000000000000 100 0 0 10 0";
        let mut inodes = HashSet::new();
        inodes.insert("739149".to_string());

        let ports = parse_proc_net_tcp(sample, &inodes);
        assert_eq!(ports, vec![41719]);
    }

    #[test]
    fn parses_ss_output_for_pid() {
        let sample = "LISTEN 0 4096 127.0.0.1:41719 0.0.0.0:* users:((\"language_server\",pid=1234,fd=20))\nLISTEN 0 4096 127.0.0.1:8080 0.0.0.0:* users:((\"other\",pid=5678,fd=4))";
        let ports = parse_ss_output(sample, 1234);
        assert_eq!(ports, vec![41719]);
    }

    #[tokio::test]
    async fn live_discovery_finds_antigravity_if_running() {
        // If Antigravity is running, discovery should find the connection
        if let Some(candidates) = find_antigravity_candidates() {
            assert!(!candidates.is_empty());
            let (pid, token) = &candidates[0];
            assert!(!token.is_empty());
            let ports = find_listening_ports(*pid);
            assert!(!ports.is_empty());
        }
    }
}
