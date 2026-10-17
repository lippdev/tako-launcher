#!/usr/bin/env bash
# Pacotes do Tako (Rust) para Linux e macOS (PRD D-07, MIG-040):
#   linux-x64 | linux-arm64   dist/Tako-Linux-<arch>.deb, .AppImage e .tar.gz
#   macos-arm64 | macos-x64   dist/Tako-macOS-<arch>.dmg e .zip (Tako.app; arm64 = Apple Silicon, x64 = Intel)
#
# Uso: build/rust/package.sh <alvo> <versão>
# No macOS a assinatura é ad hoc (sem conta Apple Developer não há notarização: o Gatekeeper pede "Abrir mesmo assim";
# PLAT-004). Credenciais do ScreenScraper (ADR-0022): TAKO_SCREENSCRAPER_DEV_ID/_DEV_PASSWORD/_SOFT_NAME no ambiente.
set -euo pipefail

target="${1:?uso: package.sh <alvo> <versão>}"
version="${2:?uso: package.sh <alvo> <versão>}"
version="${version#v}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
icon="$root/packaging/tako-mark.png"
udev="$root/packaging/linux/60-tako-uinput.rules"
dist="$root/dist"
work="$root/artifacts/rust/$target"

[[ "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-[0-9A-Za-z.-]+)?$ ]] || { echo "Versão inválida: $version" >&2; exit 1; }
numeric="${BASH_REMATCH[1]}.${BASH_REMATCH[2]}.${BASH_REMATCH[3]}"

case "$target" in
  linux-x64) os=Linux arch=x64 triple=x86_64-unknown-linux-gnu deb_arch=amd64 appimage_arch=x86_64 ;;
  linux-arm64) os=Linux arch=arm64 triple=aarch64-unknown-linux-gnu deb_arch=arm64 appimage_arch=aarch64 ;;
  macos-x64) os=macOS arch=x64 triple=x86_64-apple-darwin ;;
  macos-arm64) os=macOS arch=arm64 triple=aarch64-apple-darwin ;;
  *) echo "Alvo sem pacote: $target" >&2; exit 1 ;;
esac

# Pasta de trabalho nova a cada execução (artefatos de uma execução anterior não podem vazar para o pacote).
[[ -e "$work" ]] && mv "$work" "$work.old.$$" && rm -r "$work.old.$$"
mkdir -p "$work" "$dist"
echo "Compilando tako $version para $triple"
rustup target add "$triple" >/dev/null 2>&1 || true
TAKO_VERSION="$version" cargo build --release --locked -p tako --target "$triple"
bin="$root/target/$triple/release/tako"
[[ -x "$bin" ]] || { echo "Binário não encontrado: $bin" >&2; exit 1; }

linux() {
  local name="Tako-Linux-$arch"
  local desktop="[Desktop Entry]
Type=Application
Name=Tako
Comment=Game launcher for the TV and the controller
Comment[pt_BR]=Launcher de jogos para a TV e o controle
Exec=tako --open-url %u
MimeType=x-scheme-handler/tako;x-scheme-handler/sgdb;
Icon=tako
Categories=Game;
Terminal=false"

  # .tar.gz portable: a pasta Tako_Data ao lado do binário deixa os dados ali (tako-core::paths).
  mkdir -p "$work/tar/Tako/Tako_Data"
  cp "$bin" "$work/tar/Tako/tako"
  [[ -f "$udev" ]] && cp "$udev" "$work/tar/Tako/"
  tar -C "$work/tar" -czf "$dist/$name.tar.gz" Tako

  # .deb: /usr/bin/tako, atalho, ícone e a regra udev do cursor pelo controle (MIG-023). "~" ordena antes no Debian,
  # então 0.2.0~beta.1 < 0.2.0 (como no SemVer).
  local tilde='~'
  local deb_version="${version/-/$tilde}"
  local deb="$work/deb"
  mkdir -p "$deb/DEBIAN" "$deb/usr/bin" "$deb/usr/share/applications" "$deb/usr/share/icons/hicolor/256x256/apps"
  cp "$bin" "$deb/usr/bin/tako"
  printf '%s\n' "$desktop" >"$deb/usr/share/applications/tako.desktop"
  cp "$icon" "$deb/usr/share/icons/hicolor/256x256/apps/tako.png"
  if [[ -f "$udev" ]]; then
    mkdir -p "$deb/usr/lib/udev/rules.d"
    cp "$udev" "$deb/usr/lib/udev/rules.d/"
    cat >"$deb/DEBIAN/postinst" <<'POST'
#!/bin/sh
set -e
if command -v udevadm >/dev/null 2>&1; then
  udevadm control --reload-rules || true
  udevadm trigger --subsystem-match=misc || true
fi
POST
    chmod 755 "$deb/DEBIAN/postinst"
  fi
  cat >"$deb/DEBIAN/control" <<CTRL
Package: tako
Version: $deb_version
Section: games
Priority: optional
Architecture: $deb_arch
Maintainer: Tako <https://github.com/lippdev/tako-launcher>
Homepage: https://github.com/lippdev/tako-launcher
Depends: libc6 (>= 2.35), libfontconfig1, libxkbcommon0, libgl1
Recommends: pulseaudio-utils | pipewire-pulse, xdg-utils
Description: Tako game launcher
 Game launcher for the TV and the controller, the same on Windows, Linux and macOS.
CTRL
  dpkg-deb --build --root-owner-group "$deb" "$dist/$name.deb"

  # .AppImage: o appimagetool roda extraído porque o runner não tem FUSE.
  local appdir="$work/Tako.AppDir"
  mkdir -p "$appdir/usr/bin"
  cp "$bin" "$appdir/usr/bin/tako"
  printf '%s\n' "$desktop" >"$appdir/tako.desktop"
  cp "$icon" "$appdir/tako.png"
  cat >"$appdir/AppRun" <<'RUN'
#!/bin/sh
here="$(dirname "$(readlink -f "$0")")"
exec "$here/usr/bin/tako" "$@"
RUN
  chmod +x "$appdir/AppRun"
  local tool="$work/appimagetool"
  curl -fsSL -o "$tool" "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$appimage_arch.AppImage"
  chmod +x "$tool"
  ARCH="$appimage_arch" APPIMAGE_EXTRACT_AND_RUN=1 "$tool" --no-appstream "$appdir" "$dist/$name.AppImage"
}

macos() {
  local name="Tako-macOS-$arch"
  local bundle="$work/dmg/Tako.app"
  mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
  cp "$bin" "$bundle/Contents/MacOS/tako"

  local iconset="$work/tako.iconset"
  mkdir -p "$iconset"
  for size in 16 32 128 256; do
    sips -z "$size" "$size" "$icon" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    if [[ $size -le 128 ]]; then
      sips -z $((size * 2)) $((size * 2)) "$icon" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
    fi
  done
  iconutil -c icns "$iconset" -o "$bundle/Contents/Resources/tako.icns"

  cat >"$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Tako</string>
  <key>CFBundleDisplayName</key><string>Tako</string>
  <key>CFBundleIdentifier</key><string>dev.lipp.tako</string>
  <key>CFBundleExecutable</key><string>tako</string>
  <key>CFBundleIconFile</key><string>tako.icns</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$numeric</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.games</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
  codesign --force --deep --sign - "$bundle"

  ln -s /Applications "$work/dmg/Applications"
  hdiutil create -volname Tako -srcfolder "$work/dmg" -fs HFS+ -format UDZO -ov "$dist/$name.dmg" >/dev/null
  ditto -c -k --keepParent "$bundle" "$dist/$name.zip"
}

if [[ "$os" == Linux ]]; then linux; else macos; fi
echo "OK:"
ls -1 "$dist/Tako-$os-$arch".*
