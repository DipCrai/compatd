#!/bin/bash
# systemctl — OpenRC-compatible wrapper
# Installed at /usr/local/bin/systemctl (must be before systemd's in PATH)

SELF="systemctl (compatd)"

# Where compatd is
COMPATD="${COMPATD:-compatd}"
[[ -x "/usr/local/bin/compatd" ]] && COMPATD="/usr/local/bin/compatd"
[[ -x "$(dirname "$0")/compatd" ]] && COMPATD="$(dirname "$0")/compatd"

# Strip .service/.timer/.socket/.mount suffixes, remove @ (instances)
clean_name() {
    local n="$1"
    n="${n%.service}"
    n="${n%.timer}"
    n="${n%.socket}"
    n="${n%.mount}"
    n="${n%@*}"
    echo "$n"
}

# Parse --now flag, return remaining args
parse_now() {
    local NOW=0
    local ARGS=()
    for a in "$@"; do
        case "$a" in
            --now) NOW=1 ;;
            *) ARGS+=("$a") ;;
        esac
    done
    echo "$NOW|${ARGS[*]}"
}

help_text() {
    cat <<EOF
$SELF — systemctl emulation for OpenRC

Supported commands:
  start      <unit>...    → rc-service <unit> start
  stop       <unit>...    → rc-service <unit> stop
  restart    <unit>...    → rc-service <unit> restart
  status     <unit>...    → rc-service <unit> status
  enable     [--now] <unit>...  → rc-update add <unit>
  disable    [--now] <unit>...  → rc-update del <unit>
  daemon-reload            → (noop on OpenRC)
  list-units               → rc-status
  list-unit-files          → rc-update show
  is-enabled  <unit>       → check rc-update
  is-active   <unit>       → rc-service <unit> status
  cat         <unit>       → compatd systemctl cat <unit>
  poweroff                 → power off the system
  reboot                   → reboot the system
  hibernate                → hibernate the system
  --version                → compatd systemctl version
  --help                   → this text

Unknown commands are forwarded to compatd.
EOF
}

main() {
    [[ $# -eq 0 ]] && { help_text; exit 1; }

    local USER_MODE=0
    local GLOBAL_ARGS=()
    local SUBCMD=""
    local SUBCMD_ARGS=()

    # Parse global flags and subcommand
    for a in "$@"; do
        if [[ -z "$SUBCMD" ]]; then
            case "$a" in
                --user) USER_MODE=1 ;;
                --system) ;;
                --help) help_text; exit 0 ;;
                --version) $COMPATD systemctl version; exit $? ;;
                -*) GLOBAL_ARGS+=("$a") ;;
                *) SUBCMD="$a" ;;
            esac
        else
            SUBCMD_ARGS+=("$a")
        fi
    done

    [[ -z "$SUBCMD" ]] && { help_text; exit 1; }

    local RUNLEVEL="default"
    (( USER_MODE )) && RUNLEVEL="default"

    case "$SUBCMD" in
        start|stop|restart|status)
            local rc_cmd="$SUBCMD"
            [[ "$rc_cmd" == "status" ]] && rc_cmd="status"
            for unit in "${SUBCMD_ARGS[@]}"; do
                local svc
                svc=$(clean_name "$unit")
                exec rc-service "$svc" "$rc_cmd"
            done
            ;;

        enable|disable)
            local parsed
            parsed=$(parse_now "${SUBCMD_ARGS[@]}")
            local NOW="${parsed%%|*}"
            local UNITS="${parsed#*|}"
            for unit in $UNITS; do
                local svc
                svc=$(clean_name "$unit")
                if [[ "$SUBCMD" == "enable" ]]; then
                    rc-update add "$svc" "$RUNLEVEL"
                else
                    rc-update del "$svc"
                fi
                if [[ "$NOW" == "1" ]]; then
                    if [[ "$SUBCMD" == "enable" ]]; then
                        rc-service "$svc" start
                    else
                        rc-service "$svc" stop
                    fi
                fi
            done
            ;;

        daemon-reload|reload)
            # OpenRC doesn't need reload; some services need sighup
            # Forward to rc-service if unit is given
            if [[ ${#SUBCMD_ARGS[@]} -gt 0 ]]; then
                for unit in "${SUBCMD_ARGS[@]}"; do
                    local svc
                    svc=$(clean_name "$unit")
                    rc-service "$svc" reload 2>/dev/null || true
                done
            fi
            ;;

        list-units)
            rc-status "${GLOBAL_ARGS[@]}"
            ;;

        list-unit-files)
            rc-update show
            ;;

        is-enabled)
            local svc
            svc=$(clean_name "${SUBCMD_ARGS[0]}")
            if rc-update show | grep -q "\s$svc\s"; then
                echo "enabled"
                exit 0
            else
                echo "disabled"
                exit 1
            fi
            ;;

        is-active)
            local svc
            svc=$(clean_name "${SUBCMD_ARGS[0]}")
            rc-service "$svc" status >/dev/null 2>&1
            local rc=$?
            if [[ $rc -eq 0 ]]; then
                echo "active"
            else
                echo "inactive"
            fi
            exit $rc
            ;;

        cat)
            local svc
            svc=$(clean_name "${SUBCMD_ARGS[0]}")
            $COMPATD systemctl cat "$svc"
            ;;

        poweroff|reboot|hibernate)
            $COMPATD systemctl "$SUBCMD"
            ;;

        help)
            help_text
            ;;

        *)
            # Unknown command → forward to compatd
            $COMPATD systemctl "$SUBCMD" "${SUBCMD_ARGS[@]}"
            ;;
    esac
}

main "$@"
