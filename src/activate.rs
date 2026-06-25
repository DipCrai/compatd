/// Create socket(s), set LISTEN_FDS/LISTEN_PID/LISTEN_FDNAMES, exec target program
///
/// Usage:
///   compatd socket-activate --listen-stream /run/docker.sock -- dockerd -H fd://

use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;

#[derive(Debug, Default)]
pub struct SocketConfig {
    pub listen_stream: Vec<String>,
}

/// Parse command-line arguments for socket-activate
pub fn parse_config(listen_stream: &[String]) -> SocketConfig {
    SocketConfig {
        listen_stream: listen_stream.to_vec(),
    }
}

/// Create sockets and exec command with LISTEN_FDS
pub fn activate(config: &SocketConfig, cmd: &[String]) {
    let mut raw_fds: Vec<i32> = Vec::new();

    for addr in &config.listen_stream {
        match create_stream_socket(addr) {
            Ok(fd) => raw_fds.push(fd),
            Err(e) => {
                eprintln!("compatd: failed to create socket {addr}: {e}");
                std::process::exit(1);
            }
        }
    }

    if raw_fds.is_empty() {
        eprintln!("compatd: no sockets to activate");
        std::process::exit(1);
    }

    let listen_pid = std::process::id();
    let num_fds = raw_fds.len();
    let fd_names: Vec<String> = config
        .listen_stream
        .iter()
        .map(|s| s.rsplit('/').next().unwrap_or(s).to_string())
        .collect();

    unsafe {
        std::env::set_var("LISTEN_FDS", num_fds.to_string());
        std::env::set_var("LISTEN_PID", listen_pid.to_string());
        let names_str = fd_names.join(":");
        std::env::set_var("LISTEN_FDNAMES", names_str);
        std::env::set_var("NOTIFY_SOCKET", "/dev/null");
    }

    let start_fd = 3i32;
    for (i, fd) in raw_fds.into_iter().enumerate() {
        let target = start_fd + i as i32;
        if fd != target {
            unsafe {
                libc::dup2(fd, target);
                libc::close(fd);
            }
        }
    }

    let err = std::process::Command::new(&cmd[0])
        .args(&cmd[1..])
        .exec();

    eprintln!("compatd: exec failed: {err}");
    std::process::exit(1);
}

/// Create stream socket (Unix or TCP), return raw fd
fn create_stream_socket(addr: &str) -> std::io::Result<i32> {
    use std::mem::ManuallyDrop;
    use std::os::fd::AsRawFd;

    if addr.starts_with('/') || addr.starts_with('.') {
        let listener = ManuallyDrop::new(create_unix_socket(addr)?);
        Ok(listener.as_raw_fd())
    } else if let Some((host, port)) = addr.rsplit_once(':') {
        let listener = ManuallyDrop::new(create_tcp_socket(host, port)?);
        Ok(listener.as_raw_fd())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("unknown socket address: {addr}"),
        ))
    }
}

fn create_unix_socket(path: &str) -> std::io::Result<std::os::unix::net::UnixListener> {
    use std::os::unix::net::UnixListener;

    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))?;

    Ok(listener)
}

fn create_tcp_socket(host: &str, port: &str) -> std::io::Result<std::net::TcpListener> {
    let addr = format!("{host}:{port}");
    std::net::TcpListener::bind(&addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let streams = vec!["/run/test.sock".to_string()];
        let cfg = parse_config(&streams);
        assert_eq!(cfg.listen_stream.len(), 1);
        assert_eq!(cfg.listen_stream[0], "/run/test.sock");
    }
}
