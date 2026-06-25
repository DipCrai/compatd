use crate::unit::Service;

/// Generate OpenRC init script from parsed .service
pub fn generate(svc: &Service) -> String {
    let mut out = String::new();

    out.push_str("#!/sbin/openrc-run\n\n");
    if let Some(d) = &svc.description {
        out.push_str(&format!("description=\"{d}\"\n\n"));
    }

    // LD_PRELOAD for sd_notify + sd_journal → syslog
    out.push_str("export LD_PRELOAD=\"/usr/lib/compatd/libcompatd_preload.so${LD_PRELOAD:+:$LD_PRELOAD}\"\n\n");

    for env in &svc.environment {
        out.push_str(&format!("export {env}\n"));
    }
    for ef in &svc.environment_file {
        out.push_str(&format!(". \"{ef}\"\n"));
    }
    if !svc.environment.is_empty() || !svc.environment_file.is_empty() {
        out.push('\n');
    }

    let cmd = svc.exec_start.as_deref().unwrap_or("/bin/false");
    out.push_str(&format!("command=\"{cmd}\"\n"));

    if let Some(args) = extract_args(cmd) {
        out.push_str(&format!("command_args=\"{args}\"\n"));
        out.push_str(&format!("command=\"{}\"\n", cmd.split_whitespace().next().unwrap_or(cmd)));
    }

    if let Some(u) = &svc.user {
        out.push_str(&format!("command_user=\"{u}\"\n"));
    } else if let Some(g) = &svc.group {
        out.push_str(&format!("command_user=\":{g}\"\n"));
    }

    if let Some(p) = &svc.pid_file {
        out.push_str(&format!("pidfile=\"{p}\"\n"));
    }

    if let Some(w) = &svc.working_directory {
        out.push_str(&format!("directory=\"{w}\"\n"));
    }

    if let Some(r) = &svc.restart {
        if r == "always" || r == "on-failure" {
            out.push_str("supervisor=\"supervise-daemon\"\n");
        }
    }

    if let Some(d) = &svc.delegate {
        let user = svc.user.as_deref().unwrap_or("root");
        let name = svc.description.as_deref().unwrap_or("service");
        if d == "yes" || d == "container" {
            out.push_str(&format!(
                "\nstart_pre() {{\n\t\
                    ebegin \"Setting up cgroups for {name}\"\n\t\
                    /usr/local/bin/compatd cgroup --delegate --user {user} \"{name}\"\n\t\
                    eend $?\n}}\n"
            ));
        }
    }

    if let Some(reload_cmd) = &svc.exec_reload {
        out.push_str(&format!(
            "\nreload() {{\n\tebegin \"Reloading {}\"\n\t{reload_cmd}\n\teend $?\n}}\n",
            svc.description.as_deref().unwrap_or("service")
        ));
    }

    if let Some(stop_cmd) = &svc.exec_stop {
        out.push_str(&format!("\nstop() {{\n\tebegin \"Stopping {}\"\n\t{stop_cmd}\n\teend $?\n}}\n", &svc.description.as_deref().unwrap_or("service")));
    }

    out
}

/// Split command from args: "/usr/bin/foo --bar baz" → "--bar baz"
#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit;

    #[test]
    fn test_generate_simple() {
        let svc = unit::parse("\
[Unit]
Description=Test

[Service]
Type=simple
ExecStart=/usr/bin/myd
User=daemon
Environment=PATH=/usr/bin
");
        let out = generate(&svc);
        assert!(out.contains("description=\"Test\""));
        assert!(out.contains("command=\"/usr/bin/myd\""));
        assert!(out.contains("command_user=\"daemon\""));
        assert!(out.contains("export PATH=/usr/bin"));
    }

    #[test]
    fn test_generate_supervisor() {
        let svc = unit::parse("\
[Service]
ExecStart=/usr/bin/myd
Restart=always
");
        let out = generate(&svc);
        assert!(out.contains("supervisor=\"supervise-daemon\""));
    }

    #[test]
    fn test_generate_environment_file() {
        let svc = unit::parse("\
[Service]
ExecStart=/usr/bin/myd
EnvironmentFile=/etc/default/myd
");
        let out = generate(&svc);
        assert!(out.contains(r#". "/etc/default/myd""#));
    }
}

fn extract_args(cmd: &str) -> Option<String> {
    let mut parts = cmd.split_whitespace();
    let _cmd = parts.next()?;
    let rest: Vec<&str> = parts.collect();
    if rest.is_empty() { None } else { Some(rest.join(" ")) }
}
