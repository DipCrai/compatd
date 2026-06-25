/// Parse systemd .mount files → /etc/fstab entries

#[derive(Debug, Default)]
pub struct MountPoint {
    pub description: Option<String>,
    pub what: Option<String>,
    pub where_mount: Option<String>,
    pub fs_type: Option<String>,
    pub options: Option<String>,
    pub dump: i32,
    pub pass: i32,
    pub directory_mode: Option<String>,
}

pub fn parse(input: &str) -> MountPoint {
    let mut m = MountPoint::default();
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
                        m.description = Some(value.to_string());
                    }
                }
                "Mount" => match key {
                    "What" => m.what = Some(value.to_string()),
                    "Where" => m.where_mount = Some(value.to_string()),
                    "Type" => m.fs_type = Some(value.to_string()),
                    "Options" => m.options = Some(value.to_string()),
                    "Dump" => m.dump = value.parse().unwrap_or(0),
                    "PassNo" | "Pass" => m.pass = value.parse().unwrap_or(0),
                    "DirectoryMode" => m.directory_mode = Some(value.to_string()),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    m
}

/// Generate /etc/fstab line
pub fn to_fstab(m: &MountPoint) -> String {
    let what = m.what.as_deref().unwrap_or("none");
    let where_mount = m.where_mount.as_deref().unwrap_or("/mnt/unknown");
    let fs_type = m.fs_type.as_deref().unwrap_or("auto");
    let options = m.options.as_deref().unwrap_or("defaults");

    let mut out = String::new();

    if let Some(d) = &m.description {
        out.push_str(&format!("# {d}\n"));
    }
    if let Some(dm) = &m.directory_mode {
        out.push_str(&format!("# DirectoryMode={dm}\n"));
    }

    out.push_str(&format!(
        "{what}\t{where_mount}\t{fs_type}\t{options}\t{}\t{}\n",
        m.dump, m.pass
    ));

    out
}

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
    fn test_parse_basic() {
        let input = "\
[Unit]
Description=Backup drive

[Mount]
What=/dev/sdb1
Where=/mnt/backup
Type=ext4
Options=defaults,noatime
Dump=0
PassNo=2
";
        let m = parse(input);
        assert_eq!(m.description.as_deref(), Some("Backup drive"));
        assert_eq!(m.what.as_deref(), Some("/dev/sdb1"));
        assert_eq!(m.where_mount.as_deref(), Some("/mnt/backup"));
        assert_eq!(m.fs_type.as_deref(), Some("ext4"));
        assert_eq!(m.options.as_deref(), Some("defaults,noatime"));
        assert_eq!(m.dump, 0);
        assert_eq!(m.pass, 2);
    }

    #[test]
    fn test_to_fstab() {
        let input = "\
[Mount]
What=/dev/sdb1
Where=/mnt/backup
Type=ext4
Options=defaults
Dump=0
PassNo=2
";
        let m = parse(input);
        let out = to_fstab(&m);
        assert_eq!(out, "/dev/sdb1\t/mnt/backup\text4\tdefaults\t0\t2\n");
    }

    #[test]
    fn test_defaults() {
        let m = parse("[Mount]\nWhat=none\n");
        let out = to_fstab(&m);
        assert_eq!(out, "none\t/mnt/unknown\tauto\tdefaults\t0\t0\n");
    }
}
