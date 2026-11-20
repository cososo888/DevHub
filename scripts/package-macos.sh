#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$(uname -s)/$(uname -m)" == Darwin/arm64 ]] || { echo "An Apple Silicon Mac is required." >&2; exit 1; }
version=$(node -p 'require("./package.json").version')
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
[[ "$(node -p 'require("./src-tauri/tauri.conf.json").version')" == "$version" ]]
CI=true npm run tauri -- build --bundles app,dmg -- --locked
bundle=src-tauri/target/release/bundle
image="$bundle/dmg/DevHub V2_${version}_aarch64.dmg"
hdiutil verify "$image"
mountpoint=$(mktemp -d)
attached=false
cleanup() {
  if [[ "$attached" == true ]]; then hdiutil detach "$mountpoint" >/dev/null; fi
  rmdir "$mountpoint"
}
trap cleanup EXIT
hdiutil attach -readonly -nobrowse -noautoopen -mountpoint "$mountpoint" "$image" >/dev/null
attached=true
app="$mountpoint/DevHub V2.app"
codesign --verify --deep --strict "$app"
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist")" == "$version" ]]
[[ "$(lipo -archs "$app/Contents/MacOS/devhub-v2")" == arm64 ]]
[[ "$(readlink "$mountpoint/Applications")" == /Applications ]]

out="src-tauri/target/release/distribution/$version"
mkdir -p "$out"
filename="DevHub-V2-$version-macOS-arm64.dmg"
cp "$image" "$out/$filename"
(cd "$out" && shasum -a 256 "$filename" > SHA256SUMS.txt && shasum -a 256 -c SHA256SUMS.txt)
echo "Release files: $out"
