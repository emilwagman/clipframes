#!/bin/bash
# Builds Clipframes.app (Apple Silicon + Intel) and installs it to ~/Applications.
# NO_INSTALL=1 builds without touching the copy in ~/Applications (it may be in use).
# RELEASE=1 requires the Developer ID identity, which release.sh needs for notarizing.
set -euo pipefail
cd "$(dirname "$0")"

SPARKLE_VERSION=2.10.0
SPARKLE_SHA256=c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c
SPARKLE=vendor/sparkle

if [ ! -d "$SPARKLE/Sparkle.framework" ] || [ "$(cat "$SPARKLE/VERSION" 2>/dev/null)" != "$SPARKLE_VERSION" ]; then
  rm -rf "$SPARKLE" && mkdir -p "$SPARKLE"
  curl -fsSL -o "$SPARKLE/sparkle.tar.xz" \
    "https://github.com/sparkle-project/Sparkle/releases/download/$SPARKLE_VERSION/Sparkle-$SPARKLE_VERSION.tar.xz"
  echo "$SPARKLE_SHA256  $SPARKLE/sparkle.tar.xz" | shasum -a 256 -c -
  tar xf "$SPARKLE/sparkle.tar.xz" -C "$SPARKLE"
  echo "$SPARKLE_VERSION" > "$SPARKLE/VERSION"
fi

APP=build/Clipframes.app
rm -rf build
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Frameworks" build/tmp

for arch in arm64 x86_64; do
  swiftc -O -swift-version 5 -parse-as-library \
    -target "$arch-apple-macos15.0" \
    -F "$SPARKLE" -framework Sparkle -Xlinker -rpath -Xlinker @executable_path/../Frameworks \
    Sources/*.swift -o "build/tmp/Clipframes-$arch"
done
lipo -create build/tmp/Clipframes-arm64 build/tmp/Clipframes-x86_64 -output "$APP/Contents/MacOS/Clipframes"

cp Info.plist "$APP/Contents/Info.plist"
ditto "$SPARKLE/Sparkle.framework" "$APP/Contents/Frameworks/Sparkle.framework"

swift make-icon.swift build/tmp/AppIcon.iconset
iconutil -c icns build/tmp/AppIcon.iconset -o "$APP/Contents/Resources/AppIcon.icns"

# Developer ID when there is one: other Macs accept it once notarized, and the
# Screen Recording permission survives rebuilds. Then the local development
# identity, then ad hoc.
IDENTITIES=$(security find-identity -v -p codesigning)
IDENTITY=$(awk -F'"' '/Developer ID Application/ {print $2; exit}' <<<"$IDENTITIES")
if [ -z "$IDENTITY" ]; then
  [ -n "${RELEASE:-}" ] && { echo "RELEASE=1 needs a Developer ID Application identity" >&2; exit 1; }
  IDENTITY=$(awk -F'"' '/Apple Development/ {print $2; exit}' <<<"$IDENTITIES")
fi
IDENTITY=${IDENTITY:--}
TIMESTAMP=--timestamp
[ "$IDENTITY" = "-" ] && TIMESTAMP=--timestamp=none

# Sign inside out, as Sparkle's docs describe for apps outside the sandbox.
sign() { codesign --force --sign "$IDENTITY" --options runtime $TIMESTAMP "$@"; }
FW="$APP/Contents/Frameworks/Sparkle.framework/Versions/B"
sign "$FW/XPCServices/Installer.xpc"
sign --preserve-metadata=entitlements "$FW/XPCServices/Downloader.xpc"
sign "$FW/Autoupdate"
sign "$FW/Updater.app"
sign "$APP/Contents/Frameworks/Sparkle.framework"
sign "$APP"
codesign --verify --deep --strict "$APP"
echo "Signed with: $IDENTITY"

if [ -n "${NO_INSTALL:-}" ]; then echo "Built $APP"; exit 0; fi

DEST="$HOME/Applications/Clipframes.app"
if pgrep -xq Clipframes; then osascript -e 'quit app "Clipframes"' || true; sleep 1; fi
rm -rf "$DEST"
cp -R "$APP" "$DEST"
echo "Installed $DEST"
