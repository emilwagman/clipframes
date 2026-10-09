#!/bin/bash
# Builds the version in src-tauri/tauri.conf.json for macOS and Windows, and with --publish
# puts it on GitHub, where installed copies find it (releases/latest/download/latest.json).
#
#   ./release.sh "What changed"             build, sign, notarize; leaves everything in ../dist
#   ./release.sh "What changed" --publish   the same, then creates the GitHub release
#
# Needs, on this Mac:
#   - the Developer ID Application certificate in the keychain
#   - notary credentials saved as the "notary" keychain profile
#   - the update signing key in ../.secrets/updater.key. Installed copies only accept updates
#     signed with it: lose it and nobody can be updated again. Keep a copy somewhere safe.
#   - ssh access to the Windows build machine (WIN_HOST, default fleet-win) with the repo at
#     WIN_REPO, Rust and pnpm. The signing key never leaves this Mac.
# Bump "version" in src-tauri/tauri.conf.json (and Cargo.toml, package.json) first.
set -euo pipefail
cd "$(dirname "$0")"

NOTES=${1:?usage: ./release.sh "release notes" [--publish]}
PUBLISH=${2:-}
REPO=emilwagman/clipframes
WIN_HOST=${WIN_HOST:-fleet-win}
WIN_REPO=${WIN_REPO:-'C:\Users\User\Development\clipframes'}
VERSION=$(python3 -c "import json; print(json.load(open('src-tauri/tauri.conf.json'))['version'])")
TAG="v$VERSION"
OUT="../dist/$VERSION"
KEY=../.secrets/updater.key

[ -f "$KEY" ] || { echo "Missing $KEY." >&2; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "Commit your changes first." >&2; exit 1; }
[ "$(git rev-parse HEAD)" = "$(git rev-parse '@{u}')" ] || { echo "Push first: Windows builds what is on GitHub." >&2; exit 1; }
! git rev-parse -q --verify "refs/tags/$TAG" >/dev/null || { echo "$TAG already exists. Bump the version." >&2; exit 1; }
BRANCH=$(git rev-parse --abbrev-ref HEAD)
rm -rf "$OUT"; mkdir -p "$OUT"
sign() { TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY")" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" pnpm -s tauri signer sign "$1" >/dev/null; cat "$1.sig"; }

echo "macOS: building for Apple silicon and Intel…"
pnpm install --frozen-lockfile >/dev/null
# The update package is made after notarizing, so the build does not need the key.
pnpm -s tauri build --target universal-apple-darwin --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'
APP=src-tauri/target/universal-apple-darwin/release/bundle/macos/Clipframes.app

echo "macOS: notarizing…"
ditto -c -k --keepParent "$APP" "$OUT/notarize.zip"
xcrun notarytool submit "$OUT/notarize.zip" --keychain-profile notary --wait
rm "$OUT/notarize.zip"
xcrun stapler staple "$APP"
spctl --assess --type execute "$APP"

# The zip is what people download; the tar.gz is what installed copies update from.
ditto -c -k --sequesterRsrc --keepParent "$APP" "$OUT/Clipframes.zip"
COPYFILE_DISABLE=1 tar -czf "$OUT/Clipframes.app.tar.gz" -C "$(dirname "$APP")" Clipframes.app
MAC_SIG=$(sign "$OUT/Clipframes.app.tar.gz")

echo "Windows: building on ${WIN_HOST}…"
ssh -o ServerAliveInterval=30 "$WIN_HOST" "cd $WIN_REPO; git fetch -q; git checkout -q $BRANCH; git pull -q; if ((git rev-parse HEAD) -ne '$(git rev-parse HEAD)') { throw 'Windows is not on the same commit.' }; cd desktop; pnpm install --frozen-lockfile | Out-Null; '{\"bundle\":{\"createUpdaterArtifacts\":false}}' | Set-Content -Encoding ASCII \$env:TEMP\cf-release.json; pnpm -s tauri build --bundles nsis --config \$env:TEMP\cf-release.json | Out-Null; if (-not (Test-Path src-tauri\target\release\bundle\nsis\Clipframes_${VERSION}_x64-setup.exe)) { throw 'No installer was built.' }"
scp -q "$WIN_HOST:$(printf '%s' "$WIN_REPO" | tr '\\' '/')/desktop/src-tauri/target/release/bundle/nsis/Clipframes_${VERSION}_x64-setup.exe" "$OUT/Clipframes-setup.exe"
WIN_SIG=$(sign "$OUT/Clipframes-setup.exe")

BASE="https://github.com/$REPO/releases/download/$TAG"
python3 - "$OUT/latest.json" "$VERSION" "$NOTES" "$BASE" "$MAC_SIG" "$WIN_SIG" <<'PY'
import datetime, json, sys
path, version, notes, base, mac_sig, win_sig = sys.argv[1:]
mac = {"signature": mac_sig, "url": f"{base}/Clipframes.app.tar.gz"}
json.dump({
    "version": version,
    "notes": notes,
    "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": {
        "darwin-aarch64": mac,
        "darwin-x86_64": mac,
        "windows-x86_64": {"signature": win_sig, "url": f"{base}/Clipframes-setup.exe"},
    },
}, open(path, "w"), indent=2)
PY
rm -f "$OUT"/*.sig
ls -lh "$OUT"

if [ "$PUBLISH" != "--publish" ]; then
  echo "Built $VERSION in $OUT. Nothing was published; run again with --publish to release it."
  exit 0
fi
gh release create "$TAG" "$OUT/Clipframes.zip" "$OUT/Clipframes.app.tar.gz" "$OUT/Clipframes-setup.exe" "$OUT/latest.json" \
  --repo "$REPO" --target "$(git rev-parse HEAD)" --title "Clipframes $VERSION" --notes "$NOTES"
echo "Released $VERSION."
