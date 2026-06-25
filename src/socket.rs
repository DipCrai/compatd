/// Parse systemd .socket files → s6 run script

#[derive(Debug, Default)]
pub struct Socket {
    pub description: Option<String>,
    pub listen_stream: Vec<String>,
    pub listen_datagram: Vec<String>,
    pub listen_seq_packet: Vec<String>,
    pub listen_fifo: Vec<String>,
    pub socket_user: Option<String>,
    pub socket_group: Option<String>,
    pub socket_mode: Option<String>,
    pub service: Option<String>,
}

pub fn parse(input: &str) -> Socket {
    let mut s = Socket::default();
    let mut current_section = String::new();

    for line in join_continuations(input).lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            current_section = t[1..t.len() - 1].to_string();
            continue;
        }
        if let Some((key, value)) = t.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match current_section.as_str() {
                "Unit" => {
                    if key == "Description" {
                        s.description = Some(value.to_string());
                    }
                }
                "Socket" => match key {
                    "ListenStream" => s.listen_stream.push(value.to_string()),
                    "ListenDatagram" => s.listen_datagram.push(value.to_string()),
                    "ListenSequentialPacket" => s.listen_seq_packet.push(value.to_string()),
                    "ListenFIFO" => s.listen_fifo.push(value.to_string()),
                    "SocketUser" => s.socket_user = Some(value.to_string()),
                    "SocketGroup" => s.socket_group = Some(value.to_string()),
                    "SocketMode" => s.socket_mode = Some(value.to_string()),
                    "Service" => s.service = Some(value.to_string()),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    s
}

/// Generate s6 service directory run script
pub fn to_s6(s: &Socket) -> String {
    let mut out = String::new();

    if let Some(d) = &s.description {
        out.push_str(&format!("# {d}\n"));
    }
    if let Some(svc) = &s.service {
        out.push_str(&format!("# Activates: {svc}\n"));
    }
    out.push_str("# s6 service directory — run script\n");
    out.push_str("# Place this file at: /etc/s6/servicedir/<name>/run\n");
    out.push_str("# chmod +x run\n\n");

    for lf in &s.listen_fifo {
        out.push_str(&format!(
            "# ListenFIFO={lf}: use a separate service with:\n\
             #   mkfifo -m {mode} {lf}\n\
             #   cat {lf} | your-program\n\n",
            mode = s.socket_mode.as_deref().unwrap_or("600"),
        ));
    }

    for ld in &s.listen_datagram {
        if ld.starts_with('/') {
            out.push_str(&format!(
                "# ListenDatagram={ld}: s6-udpserver not yet supported\n"
            ));
        } else {
            out.push_str(&format!(
                "# ListenDatagram={ld}: use s6-udpserver with -B flag\n"
            ));
        }
    }

    if !s.listen_seq_packet.is_empty() {
        out.push_str("# ListenSequentialPacket: mapped as stream\n");
    }

    if let Some(ls) = s.listen_stream.first() {
        out.push_str("#!/bin/execlineb -P\n\n");

        if let Some(u) = &s.socket_user {
            out.push_str(&format!("s6-setuidgid {u}\n"));
        }

        let handler = s.service.as_deref().unwrap_or("your-service-program");
        if ls.starts_with('/') || ls.starts_with('.') {
            out.push_str(&format!(
                "s6-ipcserver -l0 -- {ls} ./{handler}\n"
            ));
        } else if let Some((host, port)) = ls.rsplit_once(':') {
            out.push_str(&format!(
                "s6-tcpserver -b 128 -B \"{host}\" {port} ./{handler}\n"
            ));
        } else {
            out.push_str(&format!(
                "s6-ipcserver -l0 -- {ls} ./{handler}\n"
            ));
        }
    }

    if out.ends_with('\n') {
        out.pop();
    }

    out
}

/// Join backslash-continued lines
fn join_continuations(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut carry = String::new();

    for line in input.lines() {
        if let Some(rest) = line.strip_suffix('\\') {
            carry.push_str(rest.trim());
            carry.push(' ');
        } else if carry.is_empty() {
            result.push_str(line);
            result.push('\n');
        } else {
            carry.push_str(line.trim_start());
            result.push_str(&carry);
            result.push('\n');
            carry.clear();
        }
    }

    if !carry.is_empty() {
        result.push_str(carry.trim_end());
        result.push('\n');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unix_socket() {
        let input = "\
[Unit]
Description=Docker socket

[Socket]
ListenStream=/run/docker.sock
SocketMode=0660
SocketUser=root
SocketGroup=docker
";
        let s = parse(input);
        assert_eq!(s.description.as_deref(), Some("Docker socket"));
        assert_eq!(s.listen_stream.len(), 1);
        assert_eq!(s.listen_stream[0], "/run/docker.sock");
        assert_eq!(s.socket_mode.as_deref(), Some("0660"));
        assert_eq!(s.socket_group.as_deref(), Some("docker"));
    }

    #[test]
    fn test_parse_tcp_socket() {
        let input = "\
[Socket]
ListenStream=0.0.0.0:8080
";
        let s = parse(input);
        assert_eq!(s.listen_stream.len(), 1);
        assert_eq!(s.listen_stream[0], "0.0.0.0:8080");
    }

    #[test]
    fn test_to_s6_unix() {
        let input = "\
[Socket]
ListenStream=/run/app.sock
SocketUser=myapp
SocketMode=0660
";
        let s = parse(input);
        let out = to_s6(&s);
        assert!(out.contains("s6-setuidgid myapp"));
        assert!(out.contains("s6-ipcserver"));
        assert!(out.contains("/run/app.sock"));
    }

    #[test]
    fn test_to_s6_tcp() {
        let input = "\
[Socket]
ListenStream=127.0.0.1:3000
";
        let s = parse(input);
        let out = to_s6(&s);
        assert!(out.contains("127.0.0.1"));
        assert!(out.contains("3000"));
    }
}
