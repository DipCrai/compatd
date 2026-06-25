#[derive(Debug, Default)]
pub struct Service {
    pub description: Option<String>,
    pub service_type: Option<String>,
    pub exec_start: Option<String>,
    pub exec_stop: Option<String>,
    pub exec_reload: Option<String>,
    pub user: Option<String>,
    pub group: Option<String>,
    pub working_directory: Option<String>,
    pub environment: Vec<String>,
    pub environment_file: Vec<String>,
    pub pid_file: Option<String>,
    pub restart: Option<String>,
    pub delegate: Option<String>,
}

pub fn parse(input: &str) -> Service {
    let mut svc = Service::default();
    let mut current_section = String::new();

    for line in join_continuations(input).lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed[1..trimmed.len() - 1].to_string();
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match current_section.as_str() {
                "Unit" => match key {
                    "Description" => svc.description = Some(value.to_string()),
                    _ => {}
                },
                "Service" => match key {
                    "Type" => svc.service_type = Some(value.to_string()),
                    "ExecStart" => svc.exec_start = Some(unescape_value(value)),
                    "ExecStop" => svc.exec_stop = Some(unescape_value(value)),
                    "ExecReload" => svc.exec_reload = Some(unescape_value(value)),
                    "User" => svc.user = Some(value.to_string()),
                    "Group" => svc.group = Some(value.to_string()),
                    "WorkingDirectory" => svc.working_directory = Some(value.to_string()),
                    "Environment" => {
                        for env in parse_env_line(value) {
                            svc.environment.push(env);
                        }
                    }
                    "EnvironmentFile" => svc.environment_file.push(value.to_string()),
                    "PIDFile" => svc.pid_file = Some(value.to_string()),
                    "Restart" => svc.restart = Some(value.to_string()),
                    "Delegate" => svc.delegate = Some(value.to_string()),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    svc
}

/// "ExecStart=/usr/bin/foo \\\n    --bar" → "ExecStart=/usr/bin/foo --bar"
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

/// Supports quotes: Environment="FOO=a b" BAR=1
fn parse_env_line(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut chars = input.chars().peekable();
    let mut current = String::new();
    let mut in_quote = false;

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quote = !in_quote;
            }
            ' ' | '\t' if !in_quote => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

/// Strip quotes from ExecStart and similar values
fn unescape_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    let mut in_quote = false;

    while let Some(c) = chars.next() {
        match c {
            '"' => in_quote = !in_quote,
            '\\' => {
                match chars.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some(x) => {
                        out.push('\\');
                        out.push(x);
                    }
                    None => out.push('\\'),
                }
            }
            _ => out.push(c),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let input = "\
[Unit]
Description=My test

[Service]
Type=simple
ExecStart=/usr/bin/foo --bar
User=testuser
Group=testgroup
";
        let svc = parse(input);
        assert_eq!(svc.description.as_deref(), Some("My test"));
        assert_eq!(svc.service_type.as_deref(), Some("simple"));
        assert_eq!(svc.exec_start.as_deref(), Some("/usr/bin/foo --bar"));
        assert_eq!(svc.user.as_deref(), Some("testuser"));
        assert_eq!(svc.group.as_deref(), Some("testgroup"));
    }

    #[test]
    fn test_parse_environment() {
        let input = "\
[Service]
Environment=FOO=bar
Environment=BAZ=123
";
        let svc = parse(input);
        assert_eq!(svc.environment.len(), 2);
        assert_eq!(svc.environment[0], "FOO=bar");
        assert_eq!(svc.environment[1], "BAZ=123");
    }

    #[test]
    fn test_environment_quoted() {
        let input = r#"
[Service]
Environment="FOO=a b" BAR=1
"#;
        let svc = parse(input);
        assert_eq!(svc.environment.len(), 2);
        assert_eq!(svc.environment[0], "FOO=a b");
        assert_eq!(svc.environment[1], "BAR=1");
    }

    #[test]
    fn test_continuation() {
        let input = "\
[Service]
ExecStart=/usr/bin/foo \
    --bar \
    --baz
";
        let svc = parse(input);
        assert_eq!(svc.exec_start.as_deref(), Some("/usr/bin/foo --bar --baz"));
    }

    #[test]
    fn test_environment_file() {
        let input = "\
[Service]
EnvironmentFile=/etc/default/foo
EnvironmentFile=/etc/foo/env
";
        let svc = parse(input);
        assert_eq!(svc.environment_file.len(), 2);
        assert_eq!(svc.environment_file[0], "/etc/default/foo");
        assert_eq!(svc.environment_file[1], "/etc/foo/env");
    }

    #[test]
    fn test_unescape() {
        let input = r#"
[Service]
ExecStart=/usr/bin/foo "arg with space"
"#;
        let svc = parse(input);
        assert_eq!(svc.exec_start.as_deref(), Some("/usr/bin/foo arg with space"));
    }

    #[test]
    fn test_parse_empty() {
        let svc = parse("");
        assert!(svc.description.is_none());
        assert!(svc.exec_start.is_none());
    }

    #[test]
    fn test_join_continuations() {
        let input = "ExecStart=/usr/bin/foo \\\n    --bar \\\n    --baz\nType=simple\n";
        let out = join_continuations(input);
        assert_eq!(out, "ExecStart=/usr/bin/foo --bar --baz\nType=simple\n");
    }
}
