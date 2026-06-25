#!/bin/bash
set -e

PREFIX="${PREFIX:-/usr/local}"
BINDIR="${DESTDIR}${PREFIX}/bin"
LIBDIR="${DESTDIR}${PREFIX}/lib/compatd"
HOOKDIR="${DESTDIR}/etc/pacman.d/hooks"

COMPATD_BIN="target/release/compatd"
PRELOAD_SO="target/release/libcompatd_preload.so"
SYSTEMCTL="systemctl.sh"
POST_SCRIPT="scripts/compatd-post.sh"
PACMAN_HOOK="scripts/compatd.hook"

echo "compatd install — systemd compatibility layer for OpenRC"
echo

if [ ! -f "$COMPATD_BIN" ]; then
    echo "Building compatd..."
    cargo build --release
fi

install -d "$BINDIR" "$LIBDIR" "$HOOKDIR"

install -m 755 "$COMPATD_BIN" "$BINDIR/compatd"
install -m 755 "$PRELOAD_SO" "$LIBDIR/libcompatd_preload.so"
install -m 755 "$SYSTEMCTL" "$BINDIR/systemctl"
install -m 755 "$POST_SCRIPT" "$BINDIR/compatd-post.sh"
install -m 644 "$PACMAN_HOOK" "$HOOKDIR/compatd.hook"

# libsystemd.so -> libelogind.so.0 (if not already)
if [ ! -L /usr/lib/libsystemd.so.0 ]; then
    echo "Creating symlink: /usr/lib/libsystemd.so.0 -> libelogind.so.0"
    ln -s libelogind.so.0 /usr/lib/libsystemd.so.0 2>/dev/null || true
fi

echo
echo "Installed to $BINDIR"
echo "  compatd          — converter + systemctl subcommands"
echo "  systemctl        — bash wrapper for OpenRC"
echo "  compatd-post.sh  — post-transaction hook for pacman"
echo
echo "Pacman hook: $HOOKDIR/compatd.hook"
echo "Preload lib: $LIBDIR/libcompatd_preload.so"
echo
echo "All done."
