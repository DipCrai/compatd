/// Parse systemd .timer files → cron entries

#[derive(Debug, Default)]
pub struct Timer {
    pub description: Option<String>,
    pub on_calendar: Vec<String>,
    pub on_boot_sec: Option<String>,
    pub on_unit_active_sec: Option<String>,
    pub on_unit_inactive_sec: Option<String>,
    pub persistent: bool,
}

pub fn parse(input: &str) -> Timer {
    let mut timer = Timer::default();
    let mut current_section = String::new();

    for line in input.lines() {
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
                        timer.description = Some(value.to_string());
                    }
                }
                "Timer" => match key {
                    "OnCalendar" => timer.on_calendar.push(value.to_string()),
                    "OnBootSec" => timer.on_boot_sec = Some(value.to_string()),
                    "OnUnitActiveSec" => timer.on_unit_active_sec = Some(value.to_string()),
                    "OnUnitInactiveSec" => timer.on_unit_inactive_sec = Some(value.to_string()),
                    "Persistent" => timer.persistent = value == "yes" || value == "true" || value == "1",
                    _ => {}
                },
                _ => {}
            }
        }
    }

    timer
}

/// Convert .timer to cron string(s)
pub fn to_cron(t: &Timer) -> Vec<String> {
    let mut lines = Vec::new();

    if let Some(d) = &t.description {
        lines.push(format!("# {d}"));
    }

    for cal in &t.on_calendar {
        if let Some(cron) = on_calendar_to_cron(cal) {
            lines.push(cron);
        }
    }

    if let Some(boot) = &t.on_boot_sec {
        let delay_sec = parse_duration(boot);
        let suffix = if delay_sec > 0 {
            format!(" sleep {}", delay_sec)
        } else {
            String::new()
        };
        lines.push(format!("@reboot{suffix} /usr/bin/systemd-timer-compat \"$@\""));
    }

    lines
}

/// Try to convert systemd OnCalendar → cron expression
fn on_calendar_to_cron(cal: &str) -> Option<String> {
    match cal {
        "minutely" | "*-*-* *:*:00" => Some("* * * * *".to_string()),
        "hourly" | "*-*-* *:00:00" => Some("0 * * * *".to_string()),
        "daily" | "*-*-* 00:00:00" => Some("0 0 * * *".to_string()),
        "weekly" | "Mon *-*-* 00:00:00" => Some("0 0 * * 1".to_string()),
        "monthly" | "*-*-01 00:00:00" => Some("0 0 1 * *".to_string()),
        "yearly" | "*-01-01 00:00:00" => Some("0 0 1 1 *".to_string()),
        _ => {
            // Try to parse as "day_of_week *-*-* HH:MM:SS" or "*:0/15"
            if let Some(rest) = cal.strip_prefix("*:") {
                // *:0/15 → */15 * * * *
                if let Some(freq) = rest.strip_prefix("0/") {
                    return Some(format!("*/{freq} * * * *"));
                }
            }
            // Couldn't parse — emit as comment
            Some(format!("# Unsupported OnCalendar: {cal}"))
        }
    }
}

/// Parse duration strings like "5min" → 300, "1h" → 3600
fn parse_duration(s: &str) -> u64 {
    let s = s.trim();
    if let Some(v) = s.strip_suffix("min") {
        v.parse::<u64>().unwrap_or(0) * 60
    } else if let Some(v) = s.strip_suffix("h") {
        v.parse::<u64>().unwrap_or(0) * 3600
    } else if let Some(v) = s.strip_suffix("s") {
        v.parse::<u64>().unwrap_or(0)
    } else if let Some(v) = s.strip_suffix("m") {
        v.parse::<u64>().unwrap_or(0) * 60
    } else if let Some(v) = s.strip_suffix("ms") {
        v.parse::<u64>().unwrap_or(0) / 1000
    } else {
        s.parse::<u64>().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timer_basic() {
        let input = "\
[Unit]
Description=Daily cleanup

[Timer]
OnCalendar=daily
Persistent=true
";
        let t = parse(input);
        assert_eq!(t.description.as_deref(), Some("Daily cleanup"));
        assert!(t.on_calendar.contains(&"daily".to_string()));
        assert!(t.persistent);
    }

    #[test]
    fn test_cron_conversion() {
        assert_eq!(on_calendar_to_cron("daily"), Some("0 0 * * *".to_string()));
        assert_eq!(on_calendar_to_cron("hourly"), Some("0 * * * *".to_string()));
        assert_eq!(on_calendar_to_cron("weekly"), Some("0 0 * * 1".to_string()));
        assert_eq!(on_calendar_to_cron("minutely"), Some("* * * * *".to_string()));
    }

    #[test]
    fn test_cron_every_15() {
        assert_eq!(on_calendar_to_cron("*:0/15"), Some("*/15 * * * *".to_string()));
    }

    #[test]
    fn test_duration() {
        assert_eq!(parse_duration("5min"), 300);
        assert_eq!(parse_duration("1h"), 3600);
        assert_eq!(parse_duration("30s"), 30);
    }
}
