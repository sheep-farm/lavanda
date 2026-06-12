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
sha256sums=('c896acdcfd6bee3d81890a36d3f38f7c53de7f74cd0f6fe72feeb61adc2df384')

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
