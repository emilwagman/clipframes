#!/bin/bash
# Builds Clipframes.app and installs it to ~/Applications.
set -euo pipefail
cd "$(dirname "$0")"

APP=build/Clipframes.app
rm -rf build
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" build/tmp

swiftc -O -swift-version 5 -parse-as-library \
  -target arm64-apple-macos15.0 \
  Sources/*.swift -o "$APP/Contents/MacOS/Clipframes"

cp Info.plist "$APP/Contents/Info.plist"

swift make-icon.swift build/tmp/AppIcon.iconset
iconutil -c icns build/tmp/AppIcon.iconset -o "$APP/Contents/Resources/AppIcon.icns"

# Sign with the local development identity when there is one, so the
# Screen Recording permission survives rebuilds. Otherwise sign ad hoc.
IDENTITY=$(security find-identity -v -p codesigning | awk '/Apple Development/ {print $2; exit}')
codesign --force --sign "${IDENTITY:--}" --options runtime "$APP"

# NO_INSTALL=1 builds without touching the copy in ~/Applications (it may be in use).
if [ -n "${NO_INSTALL:-}" ]; then echo "Built $APP"; exit 0; fi

DEST="$HOME/Applications/Clipframes.app"
if pgrep -xq Clipframes; then osascript -e 'quit app "Clipframes"' || true; sleep 1; fi
rm -rf "$DEST"
cp -R "$APP" "$DEST"
echo "Installed $DEST"
