#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_ROOT="$ROOT_DIR/target"
UNIVERSAL_BUILD_DIR="$TARGET_ROOT/universal-apple-darwin"
APP_NAME="LocalPorter"
APP_BUNDLE_DIR="$UNIVERSAL_BUILD_DIR/bundle/macos/$APP_NAME.app"
APP_VERSION="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)"
DMG_NAME="${APP_NAME}-${APP_VERSION}-macos-universal.dmg"
DMG_DIR="$UNIVERSAL_BUILD_DIR/bundle/dmg"
DMG_PATH="$DMG_DIR/$DMG_NAME"
STAGING_DIR="$UNIVERSAL_BUILD_DIR/dmg-staging"
RW_DMG_PATH="$DMG_DIR/${APP_NAME}-${APP_VERSION}-macos-universal.rw.dmg"
ICON_ICNS="$UNIVERSAL_BUILD_DIR/app-icon.icns"
ATTACHED_DEVICE=""
ATTACHED_MOUNTPOINT=""
CAN_APPLY_DMG_FILE_ICON="false"

assert_command_available() {
  local command_name="$1"
  local install_hint="$2"

  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "$command_name was not found in PATH. $install_hint" >&2
    exit 1
  fi
}

assert_path_exists() {
  local target_path="$1"
  local description="$2"

  if [[ ! -e "$target_path" ]]; then
    echo "$description was not found: $target_path" >&2
    exit 1
  fi
}

check_optional_dmg_icon_tools() {
  if command -v Rez >/dev/null 2>&1 && command -v DeRez >/dev/null 2>&1 && command -v SetFile >/dev/null 2>&1; then
    CAN_APPLY_DMG_FILE_ICON="true"
  else
    echo "Warning: Rez/DeRez/SetFile not found. The mounted volume will use a custom icon, but the .dmg file itself may keep the default Finder icon." >&2
  fi
}

preflight_checks() {
  assert_command_available "hdiutil" "This script must run on macOS."
  assert_command_available "cp" "Install macOS command line tools before building the DMG."
  assert_command_available "ln" "Install macOS command line tools before building the DMG."
  assert_command_available "xattr" "Install macOS command line tools before building the DMG."

  assert_path_exists "$ROOT_DIR/scripts/build-macos-app.sh" "macOS app build script"

  check_optional_dmg_icon_tools
}

build_app_bundle() {
  "$ROOT_DIR/scripts/build-macos-app.sh"
  assert_path_exists "$APP_BUNDLE_DIR" "Built macOS app bundle"
  assert_path_exists "$ICON_ICNS" "Generated ICNS icon"
}

prepare_staging_dir() {
  rm -rf "$STAGING_DIR"
  mkdir -p "$STAGING_DIR"

  cp -R "$APP_BUNDLE_DIR" "$STAGING_DIR/$APP_NAME.app"
  ln -s /Applications "$STAGING_DIR/Applications"
}

create_rw_dmg() {
  rm -rf "$DMG_DIR"
  mkdir -p "$DMG_DIR"

  hdiutil create \
    -volname "$APP_NAME" \
    -srcfolder "$STAGING_DIR" \
    -ov \
    -format UDRW \
    "$RW_DMG_PATH" >/dev/null
}

attach_rw_dmg() {
  local attach_output

  attach_output="$(hdiutil attach "$RW_DMG_PATH" -readwrite -noverify -noautoopen)"
  ATTACHED_DEVICE="$(printf '%s\n' "$attach_output" | awk '/\/Volumes\// {print $1}' | tail -n 1)"
  ATTACHED_MOUNTPOINT="$(printf '%s\n' "$attach_output" | awk '/\/Volumes\// {print $NF}' | tail -n 1)"

  if [[ -z "$ATTACHED_DEVICE" || -z "$ATTACHED_MOUNTPOINT" ]]; then
    echo "Failed to detect mounted DMG device or mount point." >&2
    exit 1
  fi
}

set_custom_icon_flag() {
  local target_path="$1"

  if command -v SetFile >/dev/null 2>&1; then
    SetFile -a C "$target_path"
    return
  fi

  xattr -wx com.apple.FinderInfo \
    "0000000000000000040000000000000000000000000000000000000000000000" \
    "$target_path"
}

apply_volume_icon() {
  cp "$ICON_ICNS" "$ATTACHED_MOUNTPOINT/.VolumeIcon.icns"
  set_custom_icon_flag "$ATTACHED_MOUNTPOINT"
}

detach_rw_dmg() {
  if [[ -n "$ATTACHED_DEVICE" ]]; then
    hdiutil detach "$ATTACHED_DEVICE" >/dev/null || hdiutil detach "$ATTACHED_DEVICE" -force >/dev/null
    ATTACHED_DEVICE=""
    ATTACHED_MOUNTPOINT=""
  fi
}

build_final_dmg() {
  rm -f "$DMG_PATH"

  hdiutil convert \
    "$RW_DMG_PATH" \
    -ov \
    -format UDZO \
    -imagekey zlib-level=9 \
    -o "$DMG_PATH" >/dev/null
}

apply_dmg_file_icon() {
  local rez_script

  if [[ "$CAN_APPLY_DMG_FILE_ICON" != "true" ]]; then
    return
  fi

  rez_script="$DMG_DIR/dmg-icon.r"

  sips -i "$ICON_ICNS" >/dev/null
  DeRez -only icns "$ICON_ICNS" >"$rez_script"
  Rez -append "$rez_script" -o "$DMG_PATH"
  SetFile -a C "$DMG_PATH"
  rm -f "$rez_script"
}

cleanup() {
  detach_rw_dmg
}

main() {
  trap cleanup EXIT

  preflight_checks
  build_app_bundle
  prepare_staging_dir
  create_rw_dmg
  attach_rw_dmg
  apply_volume_icon
  detach_rw_dmg
  build_final_dmg
  apply_dmg_file_icon

  rm -f "$RW_DMG_PATH"

  echo "Built macOS DMG with custom volume icon:"
  echo "  $DMG_PATH"
}

main "$@"
