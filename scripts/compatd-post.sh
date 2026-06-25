#!/bin/bash
# compatd-post-transaction — called after pacman
# Finds new .service files and converts to OpenRC

COMPATD="${COMPATD:-/usr/bin/compatd}"
CONVERTED_DIR="${COMPATD_CONVERTED_DIR:-/etc/init.d}"
LOGFILE="/var/log/compatd-hook.log"

log() {
    echo "[$(date '+%F %T')] $*" >> "$LOGFILE"
}

# Find .service files not yet in /etc/init.d/
find /usr/lib/systemd/system -name '*.service' 2>/dev/null | while read -r svc; do
    name=$(basename "$svc" .service)
    dest="${CONVERTED_DIR}/${name}"

    # Skip if OpenRC script already exists
    [[ -x "$dest" ]] && continue

    # Skip systemd-only services
    case "$name" in
        systemd-*|syslog*|tmpfiles*|journald*|logind*|resolved*|timesyncd*)
            continue ;;
    esac

    log "Converting: $name ($svc)"

    if "$COMPATD" convert service "$svc" > "$dest"; then
        chmod +x "$dest"
        log "OK: $name -> $dest"
    else
        log "FAIL: $name conversion failed"
        rm -f "$dest"
    fi
done
