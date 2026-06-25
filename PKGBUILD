# Maintainer: dipcrai <dipcrai@users.noreply.github.com>

pkgname=compatd
pkgver=0.1.0
pkgrel=1
pkgdesc="systemd compatibility layer for OpenRC — convert .service/.timer/.socket/.mount to native formats, LD_PRELOAD shim for sd_notify/sd_journal, systemctl wrapper"
arch=('x86_64')
url="https://github.com/DipCrai/compatd"
license=('MIT')
depends=('glibc' 'elogind')
makedepends=('cargo')
optdepends=(
    's6: socket activation (.socket → s6)'
    'cronie: timer conversion (.timer → cron)'
)
install=compatd.install
source=("$pkgname-$pkgver.tar.gz::https://github.com/DipCrai/compatd/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$srcdir/$pkgname-$pkgver"
    cargo build --release --frozen
}

package() {
    cd "$srcdir/$pkgname-$pkgver"

    # Binary + preload shim
    install -Dm755 target/release/compatd "$pkgdir/usr/bin/compatd"
    install -Dm755 target/release/libcompatd_preload.so \
        "$pkgdir/usr/lib/compatd/libcompatd_preload.so"

    # systemctl wrapper
    install -Dm755 systemctl.sh "$pkgdir/usr/bin/systemctl"

    # Pacman hook
    install -Dm644 scripts/compatd.hook \
        "$pkgdir/etc/pacman.d/hooks/compatd.hook"
    install -Dm755 scripts/compatd-post.sh \
        "$pkgdir/usr/bin/compatd-post.sh"

    # License
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/compatd/LICENSE"

    # Symlink libsystemd.so.0 -> libelogind.so.0
    mkdir -p "$pkgdir/usr/lib"
    ln -s libelogind.so.0 "$pkgdir/usr/lib/libsystemd.so.0"
}
