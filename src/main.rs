use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;

mod unit;
mod openrc;
mod timer;
mod socket;
mod mount;
mod activate;
mod cgroup;

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Convert {
        #[clap(subcommand)]
        kind: ConvertKind,
    },
    Systemctl {
        #[clap(subcommand)]
        action: SystemctlAction,
    },
    Cgroup {
        name: String,
        #[arg(long, default_value = "")]
        path: String,
        #[arg(long)]
        delegate: bool,
        #[arg(long)]
        user: Option<String>,
    },
    SocketActivate {
        #[arg(long = "listen-stream", short = 'l')]
        listen_stream: Vec<String>,
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
}

#[derive(Subcommand)]
enum ConvertKind {
    Service { path: String },
    Timer { path: String },
    Socket { path: String },
    Mount { path: String },
}

#[derive(Subcommand)]
enum SystemctlAction {
    Cat { name: String },
    Version,
    Whereis { name: String },
    Poweroff,
    Reboot,
    Hibernate,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Convert { kind } => match kind {
            ConvertKind::Service { path } => {
                let content = read_file(&path);
                let svc = unit::parse(&content);
                let script = openrc::generate(&svc);
                println!("{script}");
            }
            ConvertKind::Timer { path } => {
                let content = read_file(&path);
                let t = timer::parse(&content);
                for line in timer::to_cron(&t) {
                    println!("{line}");
                }
            }
            ConvertKind::Socket { path } => {
                let content = read_file(&path);
                let s = socket::parse(&content);
                print!("{}", socket::to_s6(&s));
            }
            ConvertKind::Mount { path } => {
                let content = read_file(&path);
                let m = mount::parse(&content);
                print!("{}", mount::to_fstab(&m));
            }
        },
        Command::Cgroup { name, path, delegate, user } => {
            let cg_path = if path.is_empty() {
                format!("/sys/fs/cgroup/system.slice/{name}")
            } else {
                path.clone()
            };
            let config = cgroup::CgroupConfig {
                path: cg_path,
                delegate,
                user,
            };
            cgroup::setup(&config);
        }
        Command::SocketActivate { listen_stream, command } => {
            let config = activate::parse_config(&listen_stream);
            activate::activate(&config, &command);
        }
        Command::Systemctl { action } => match action {
            SystemctlAction::Cat { name } => cmd_cat(&name),
            SystemctlAction::Version => {
                println!("compatd 0.1.0 (systemd compatibility layer for OpenRC)");
            }
            SystemctlAction::Whereis { name } => cmd_whereis(&name),
            SystemctlAction::Poweroff => cmd_poweroff(),
            SystemctlAction::Reboot => cmd_reboot(),
            SystemctlAction::Hibernate => cmd_hibernate(),
        },
    }
}

fn cmd_cat(name: &str) {
    let paths = [
        format!("/usr/lib/systemd/system/{name}.service"),
        format!("/etc/systemd/system/{name}.service"),
        format!("/run/systemd/system/{name}.service"),
    ];

    for p in &paths {
        if Path::new(p).exists() {
            let content = read_file(p);
            let svc = unit::parse(&content);
            let script = openrc::generate(&svc);
            println!("# Converted from {p}");
            println!("{script}");
            return;
        }
    }

    eprintln!("compatd: service '{name}' not found in any systemd path");
    std::process::exit(1);
}

fn cmd_whereis(name: &str) {
    let paths = [
        format!("/usr/lib/systemd/system/{name}.service"),
        format!("/etc/systemd/system/{name}.service"),
        format!("/run/systemd/system/{name}.service"),
    ];

    for p in &paths {
        if Path::new(&p).exists() {
            println!("{p}");
            return;
        }
    }
}

fn read_file(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading {path}: {e}");
        std::process::exit(1);
    })
}

fn cmd_poweroff() {
    unsafe { libc::sync() }
    let ret = unsafe { libc::reboot(libc::LINUX_REBOOT_CMD_POWER_OFF) };
    if ret != 0 {
        eprintln!("compatd: poweroff failed: {}", std::io::Error::last_os_error());
        std::process::exit(1);
    }
}

fn cmd_reboot() {
    unsafe { libc::sync() }
    let ret = unsafe { libc::reboot(libc::LINUX_REBOOT_CMD_RESTART) };
    if ret != 0 {
        eprintln!("compatd: reboot failed: {}", std::io::Error::last_os_error());
        std::process::exit(1);
    }
}

fn cmd_hibernate() {
    unsafe { libc::sync() }
    std::fs::write("/sys/power/state", "disk").unwrap_or_else(|e| {
        eprintln!("compatd: hibernate failed: {e}");
        std::process::exit(1);
    });
}
