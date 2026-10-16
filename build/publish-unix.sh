#!/usr/bin/env bash
# Pacotes do Tako para Linux e macOS (o equivalente do Publish-Tako.ps1 fora do Windows):
#   linux-x64 | linux-arm64   dist/Tako-Linux-<arch>.deb, .AppImage e .tar.gz
#   osx-arm64 | osx-x64       dist/Tako-macOS-<arch>.dmg e .zip (Tako.app; arm64 = Apple Silicon, x64 = Intel)
#
# A UI atual é WPF e só roda no Windows: estes pacotes esperam um front-end multiplataforma. O projeto vem de
# TAKO_UNIX_PROJECT (variável do repositório, lida pelo release.yml); sem ela o workflow nem chama este script.
#
# Uso: build/publish-unix.sh <rid> <versão>
# Ambiente: TAKO_UNIX_PROJECT (obrigatório, .csproj), TAKO_UNIX_FRAMEWORK (padrão net10.0),
#           TAKO_UNIX_EXECUTABLE (nome do executável publicado; padrão = nome do .csproj).
# No macOS a assinatura é ad hoc (sem conta Apple Developer não há notarização: o Gatekeeper pede "Abrir mesmo assim").
set -euo pipefail

rid="${1:?uso: publish-unix.sh <rid> <versão>}"
version="${2:?uso: publish-unix.sh <rid> <versão>}"
version="${version#v}"
root="$(cd "$(dirname "$0")/.." && pwd)"
project="${TAKO_UNIX_PROJECT:?defina TAKO_UNIX_PROJECT com o .csproj do front-end multiplataforma}"
[[ "$project" = /* ]] || project="$root/$project"
framework="${TAKO_UNIX_FRAMEWORK:-net10.0}"
exe="${TAKO_UNIX_EXECUTABLE:-$(basename "$project" .csproj)}"
icon="$root/src/Launcher.UI.Wpf/Assets/Brand/tako-mark.png"
dist="$root/dist"
work="$root/artifacts/unix/$rid"

[[ "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-[0-9A-Za-z.-]+)?$ ]] || { echo "Versão inválida: $version" >&2; exit 1; }
numeric="${BASH_REMATCH[1]}.${BASH_REMATCH[2]}.${BASH_REMATCH[3]}"

case "$rid" in
  linux-x64) os=Linux arch=x64 deb_arch=amd64 appimage_arch=x86_64 ;;
  linux-arm64) os=Linux arch=arm64 deb_arch=arm64 appimage_arch=aarch64 ;;
  osx-x64) os=macOS arch=x64 ;;
  osx-arm64) os=macOS arch=arm64 ;;
  *) echo "RID sem pacote: $rid" >&2; exit 1 ;;
esac

rm -rf "$work"
mkdir -p "$work" "$dist"
app="$work/publish"
echo "Publicando $project ($framework, $rid) como $version"
dotnet publish "$project" -c Release -f "$framework" -r "$rid" --self-contained true \
  -p:Version="$version" -p:FileVersion="$numeric.0" -p:AssemblyVersion="$numeric.0" \
  -p:DebugType=None -p:DebugSymbols=false -o "$app"
[[ -x "$app/$exe" ]] || { echo "Executável $exe não encontrado em $app (ajuste TAKO_UNIX_EXECUTABLE)" >&2; exit 1; }

linux() {
  local name="Tako-Linux-$arch"

  # .tar.gz: a pasta publicada, para quem prefere sem instalar.
  mkdir -p "$work/tar"
  cp -R "$app" "$work/tar/Tako"
  tar -C "$work/tar" -czf "$dist/$name.tar.gz" Tako

  local desktop="[Desktop Entry]
Type=Application
Name=Tako
Comment=Game launcher for the TV and the controller
Exec=tako
Icon=tako
Categories=Game;
Terminal=false"

  # .deb: /opt/tako + /usr/bin/tako. No Debian "~" ordena antes, então 0.2.0~beta.1 < 0.2.0 (como no SemVer).
  local tilde='~'
  local deb_version="${version/-/$tilde}"
  local deb="$work/deb"
  mkdir -p "$deb/DEBIAN" "$deb/opt" "$deb/usr/bin" "$deb/usr/share/applications" "$deb/usr/share/icons/hicolor/256x256/apps"
  cp -R "$app" "$deb/opt/tako"
  ln -s "/opt/tako/$exe" "$deb/usr/bin/tako"
  printf '%s\n' "$desktop" >"$deb/usr/share/applications/tako.desktop"
  cp "$icon" "$deb/usr/share/icons/hicolor/256x256/apps/tako.png"
  cat >"$deb/DEBIAN/control" <<EOF
Package: tako
Version: $deb_version
Section: games
Priority: optional
Architecture: $deb_arch
Maintainer: Tako <https://github.com/lippdev/tako-launcher>
Homepage: https://github.com/lippdev/tako-launcher
Depends: libc6, libgcc-s1, libstdc++6, zlib1g, libicu74 | libicu72 | libicu71 | libicu70 | libicu67 | libicu66
Description: Tako game launcher
 Game launcher for the TV and the controller.
EOF
  dpkg-deb --build --root-owner-group "$deb" "$dist/$name.deb"

  # .AppImage: AppDir com AppRun; o appimagetool roda extraído porque o runner não tem FUSE.
  local appdir="$work/Tako.AppDir"
  mkdir -p "$appdir/usr"
  cp -R "$app" "$appdir/usr/bin"
  printf '%s\n' "$desktop" >"$appdir/tako.desktop"
  cp "$icon" "$appdir/tako.png"
  cat >"$appdir/AppRun" <<EOF
#!/bin/sh
here="\$(dirname "\$(readlink -f "\$0")")"
exec "\$here/usr/bin/$exe" "\$@"
EOF
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
  cp -R "$app/." "$bundle/Contents/MacOS/"

  # Ícone: o mesmo PNG do Linux, nos tamanhos que o iconutil aceita até 256.
  local iconset="$work/tako.iconset"
  mkdir -p "$iconset"
  for size in 16 32 128 256; do
    sips -z "$size" "$size" "$icon" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    if [[ $size -le 128 ]]; then
      sips -z $((size * 2)) $((size * 2)) "$icon" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
    fi
  done
  iconutil -c icns "$iconset" -o "$bundle/Contents/Resources/tako.icns"

  cat >"$bundle/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Tako</string>
  <key>CFBundleDisplayName</key><string>Tako</string>
  <key>CFBundleIdentifier</key><string>dev.lipp.tako</string>
  <key>CFBundleExecutable</key><string>$exe</string>
  <key>CFBundleIconFile</key><string>tako.icns</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$numeric</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.games</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
  codesign --force --deep --sign - "$bundle"

  ln -s /Applications "$work/dmg/Applications"
  hdiutil create -volname Tako -srcfolder "$work/dmg" -fs HFS+ -format UDZO -ov "$dist/$name.dmg" >/dev/null
  ditto -c -k --keepParent "$bundle" "$dist/$name.zip"
}

if [[ "$os" == Linux ]]; then linux; else macos; fi
echo "OK:"
ls -1 "$dist/Tako-$os-$arch".*
