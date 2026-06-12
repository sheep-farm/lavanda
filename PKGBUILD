# Maintainer: Flavio de Vasconcellos <flavio.de.vasconcellos@gmail.com>
pkgname=lavanda
pkgver=1.0.0
pkgrel=1
pkgdesc="Native Wayland music player for Omarchy/Hyprland with live theming and MPRIS2"
arch=('x86_64')
url="https://github.com/sheep-farm/lavanda"
license=('MIT')
depends=(
    'alsa-lib'
    'dbus'
)
optdepends=(
    'pipewire-alsa: PipeWire audio support'
    'pulseaudio-alsa: PulseAudio audio support'
    'libnotify: track change notifications (notify-send)'
    'ttf-jetbrains-mono-nerd: recommended Nerd Font'
)
makedepends=('cargo')
source=("$pkgname-$pkgver.tar.gz::https://github.com/sheep-farm/lavanda/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('10584008bdaeb27276c9500803c3ac8a8f24044ca4f83cb6213afdaf16362231')

prepare() {
    cd "$pkgname-$pkgver"
    cargo fetch --target "$CARCH-unknown-linux-gnu"
}

build() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --offline --release
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 target/release/lavanda      "$pkgdir/usr/bin/lavanda"
    install -Dm644 assets/lavanda.desktop      "$pkgdir/usr/share/applications/lavanda.desktop"
    install -Dm644 LICENSE                     "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
