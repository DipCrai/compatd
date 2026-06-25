# compatd

A lightweight systemd compatibility layer for OpenRC-based Linux distributions.

![Rust](https://img.shields.io/badge/language-Rust-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

Designed for OpenRC-based distributions such as Artix, Gentoo and Alpine Linux.

## Why?

Some packages ship `.service` files and assume systemd. On OpenRC you can either:
- Write init scripts by hand (tedious)
- Convert automatically with compatd
- Run the LD_PRELOAD shim if a binary calls `sd_journal_*` / `sd_notify`

## Features

- **systemctl-compatible wrapper** — preserves muscle memory: `start`, `stop`, `enable`, `disable`, `status`, `is-active`, `is-enabled`, `daemon-reload`, `list-units`, `cat`, `poweroff`, `reboot`, `hibernate`
- **Convert units** — `.service` → OpenRC, `.timer` → cron, `.socket` → s6, `.mount` → fstab
- **LD_PRELOAD shim** — minimal implementations of `sd_notify`, `sd_journal_*`, `sd_booted`, `sd_listen_fds`, `sd_watchdog_enabled`, `sd_id128_get_machine` for software that calls systemd APIs directly
- **Socket activation** — fd-passing for `-H fd://` style services
- **Cgroup delegation** — handles `Delegate=yes` via cgroup v2
- **Pacman hook** — converts new `.service` files automatically after package install
- **elogind integration** — seamless compatibility through `libelogind`

## Usage

### Convert units

```bash
# .service → OpenRC
compatd convert service /usr/lib/systemd/system/docker.service > /etc/init.d/docker
chmod +x /etc/init.d/docker
rc-update add docker default

# .timer → cron
compatd convert timer /usr/lib/systemd/system/daily-cleanup.timer >> /var/spool/cron/root

# .socket → s6
compatd convert socket /usr/lib/systemd/system/docker.socket > /etc/s6/docker/run

# .mount → fstab
compatd convert mount /usr/lib/systemd/system/var-data.mount >> /etc/fstab
```

### systemctl wrapper

```bash
systemctl start docker
systemctl enable docker
systemctl status docker
systemctl is-active docker
systemctl is-enabled docker
systemctl daemon-reload
systemctl list-units
systemctl cat docker
systemctl poweroff
systemctl reboot
systemctl hibernate
```

### Socket activation

```bash
compatd socket-activate --listen-stream /run/docker.sock -- dockerd -H fd://
```

## Installation

```bash
# Build from source (PKGBUILD, Artix/Arch)
git clone https://github.com/DipCrai/compatd
cd compatd && makepkg -si

# Build with Cargo (any distribution)
cargo build --release

# Local install script
./install.sh
```

## How it works

1. **Unit parser** — custom parser for `.service` / `.timer` / `.socket` / `.mount` (no external dependencies)
2. **Unit converter** — translates parsed units to OpenRC, cron, s6, fstab
3. **LD_PRELOAD shim** (`libcompatd_preload.so`) — provides minimal implementations for `sd_notify`, `sd_journal_*`, `sd_booted`, `sd_is_socket`, `sd_listen_fds`, `sd_watchdog_enabled`, `sd_id128_get_machine`; journal writes go to `syslog(3)`, reads return empty
4. **systemctl wrapper** — maps 11 `systemctl` commands to `rc-service` / `rc-update` / `compatd`
5. **Pacman hook** — post-transaction script converts newly installed `.service` files
6. **elogind** — drop-in replacement for `libsystemd` where applicable

## Limitations

compatd handles the most common unit patterns. Complex features may need manual adjustments:

- Socket activation internals (fd-passing works, but full socket unit lifecycle isn't replicated)
- Transient units (dynamically created by systemd at runtime)
- Advanced cgroup configuration beyond `Delegate=yes`
- systemd generators and portable services
- `.path` / `.slice` / `.scope` unit types

If you hit something that doesn't convert cleanly, open an issue.
