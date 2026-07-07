#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ASSETS_DIR="$ROOT_DIR/crates/localporter-ui/assets"
TARGET_DIR="$ROOT_DIR/target/release"
APP_NAME="LocalPorter"
BINARY_NAME="localporter-app"
APP_BUNDLE_ID="com.localporter.app"
BUNDLE_DIR="$TARGET_DIR/bundle/macos/$APP_NAME.app"
CONTENTS_DIR="$BUNDLE_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
ICON_PNG="$ASSETS_DIR/app-icon.png"
ICON_ICNS="$TARGET_DIR/app-icon.icns"
ICONSET_DIR="$TARGET_DIR/app-icon.iconset"
APP_VERSION="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)"

build_binary() {
  cargo build --locked --release -p "$BINARY_NAME"
}

warn_if_icon_is_small() {
  local width height

  width="$(sips -g pixelWidth "$ICON_PNG" 2>/dev/null | awk '/pixelWidth/ {print $2}')"
  height="$(sips -g pixelHeight "$ICON_PNG" 2>/dev/null | awk '/pixelHeight/ {print $2}')"

  if [[ -n "$width" && -n "$height" && ( "$width" -lt 1024 || "$height" -lt 1024 ) ]]; then
    echo "Warning: $ICON_PNG is ${width}x${height}. 1024x1024 is recommended for sharper macOS icons." >&2
  fi
}

generate_icns() {
  warn_if_icon_is_small

  rm -rf "$ICONSET_DIR"
  mkdir -p "$ICONSET_DIR"

  sips -z 16 16 "$ICON_PNG" --out "$ICONSET_DIR/icon_16x16.png" >/dev/null
  sips -z 32 32 "$ICON_PNG" --out "$ICONSET_DIR/icon_16x16@2x.png" >/dev/null
  sips -z 32 32 "$ICON_PNG" --out "$ICONSET_DIR/icon_32x32.png" >/dev/null
  sips -z 64 64 "$ICON_PNG" --out "$ICONSET_DIR/icon_32x32@2x.png" >/dev/null
  sips -z 128 128 "$ICON_PNG" --out "$ICONSET_DIR/icon_128x128.png" >/dev/null
  sips -z 256 256 "$ICON_PNG" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null
  sips -z 256 256 "$ICON_PNG" --out "$ICONSET_DIR/icon_256x256.png" >/dev/null
  sips -z 512 512 "$ICON_PNG" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null
  sips -z 512 512 "$ICON_PNG" --out "$ICONSET_DIR/icon_512x512.png" >/dev/null
  sips -z 1024 1024 "$ICON_PNG" --out "$ICONSET_DIR/icon_512x512@2x.png" >/dev/null

  iconutil -c icns "$ICONSET_DIR" -o "$ICON_ICNS"
}

prepare_bundle() {
  rm -rf "$BUNDLE_DIR"
  mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

  cp "$TARGET_DIR/$BINARY_NAME" "$MACOS_DIR/$APP_NAME"
  chmod +x "$MACOS_DIR/$APP_NAME"
  cp "$ICON_ICNS" "$RESOURCES_DIR/app-icon.icns"
}

write_info_plist() {
  cat >"$CONTENTS_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>LocalPorter</string>
  <key>CFBundleExecutable</key>
  <string>LocalPorter</string>
  <key>CFBundleIconFile</key>
  <string>app-icon</string>
  <key>CFBundleIdentifier</key>
  <string>${APP_BUNDLE_ID}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>LocalPorter</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${APP_VERSION}</string>
  <key>CFBundleVersion</key>
  <string>${APP_VERSION}</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF
}

main() {
  build_binary
  generate_icns
  prepare_bundle
  write_info_plist

  echo "Built macOS app bundle:"
  echo "  $BUNDLE_DIR"
}

main "$@"
