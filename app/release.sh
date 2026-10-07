#!/bin/bash
# Ships the version in Info.plist: builds, notarizes, publishes a GitHub release
# and adds it to appcast.xml, which installed copies check through Sparkle.
#
#   ./release.sh "What changed, one line per item"
#
# Needs, once per Mac:
#   - the Developer ID Application certificate in the keychain
#   - notary credentials saved as the "notary" profile, from the App Store Connect API key Amber Notes uses:
#     xcrun notarytool store-credentials notary --key AuthKey_<id>.p8 --key-id <id> --issuer <issuer id>
#   - the Sparkle signing key in the keychain. The backup is .secrets/sparkle-private-key.txt;
#     import it on a new Mac with vendor/sparkle/bin/generate_keys -f .secrets/sparkle-private-key.txt
# Bump CFBundleShortVersionString and CFBundleVersion in Info.plist first.
set -euo pipefail
cd "$(dirname "$0")"

NOTES=${1:?usage: ./release.sh "release notes"}
REPO=emilwagman/clipframes
VERSION=$(/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" Info.plist)
BUILD=$(/usr/libexec/PlistBuddy -c "Print :CFBundleVersion" Info.plist)
MIN_OS=$(/usr/libexec/PlistBuddy -c "Print :LSMinimumSystemVersion" Info.plist)
TAG="v$VERSION"
ZIP="../dist/Clipframes-$VERSION.zip"
APPCAST=../appcast.xml

[ -z "$(git status --porcelain)" ] || { echo "Commit your changes first." >&2; exit 1; }
! git rev-parse -q --verify "refs/tags/$TAG" >/dev/null || { echo "$TAG already exists. Bump the version in Info.plist." >&2; exit 1; }
grep -q "<sparkle:version>$BUILD</sparkle:version>" "$APPCAST" 2>/dev/null && { echo "Build $BUILD is already in appcast.xml. Bump CFBundleVersion." >&2; exit 1; }

RELEASE=1 NO_INSTALL=1 ./build.sh

echo "Notarizing…"
mkdir -p ../dist
ditto -c -k --keepParent build/Clipframes.app build/tmp/notarize.zip
xcrun notarytool submit build/tmp/notarize.zip --keychain-profile notary --wait
xcrun stapler staple build/Clipframes.app
spctl --assess --type execute build/Clipframes.app
rm -f "$ZIP"
ditto -c -k --sequesterRsrc --keepParent build/Clipframes.app "$ZIP"

SIGNATURE=$(vendor/sparkle/bin/sign_update "$ZIP")   # sparkle:edSignature="…" length="…"
URL="https://github.com/$REPO/releases/download/$TAG/Clipframes-$VERSION.zip"
DESCRIPTION=$(printf '%s\n' "$NOTES" | sed 's/^/<li>/; s/$/<\/li>/' | tr -d '\n')

ITEM="    <item>
      <title>$VERSION</title>
      <pubDate>$(LC_ALL=C date -u '+%a, %d %b %Y %H:%M:%S +0000')</pubDate>
      <sparkle:version>$BUILD</sparkle:version>
      <sparkle:shortVersionString>$VERSION</sparkle:shortVersionString>
      <sparkle:minimumSystemVersion>$MIN_OS</sparkle:minimumSystemVersion>
      <description><![CDATA[<ul>$DESCRIPTION</ul>]]></description>
      <enclosure url=\"$URL\" $SIGNATURE type=\"application/octet-stream\"/>
    </item>"

if [ ! -f "$APPCAST" ]; then
  cat > "$APPCAST" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>Clipframes</title>
    <!-- release.sh adds new versions below this line -->
  </channel>
</rss>
EOF
fi
python3 - "$APPCAST" "$ITEM" <<'EOF'
import sys
path, item = sys.argv[1], sys.argv[2]
marker = "<!-- release.sh adds new versions below this line -->"
s = open(path).read()
assert marker in s, "appcast.xml is missing the marker line"
open(path, "w").write(s.replace(marker, marker + "\n" + item, 1))
EOF

git push
# Also attached as Clipframes.zip, so releases/latest/download/Clipframes.zip (the website's link) is always the newest.
mkdir -p build/latest && cp "$ZIP" build/latest/Clipframes.zip
gh release create "$TAG" "$ZIP" build/latest/Clipframes.zip --repo "$REPO" --target "$(git rev-parse HEAD)" --title "Clipframes $VERSION" --notes "$NOTES"
git add "$APPCAST"
git commit -m "Release $VERSION"
git push

echo "Released $VERSION: https://github.com/$REPO/releases/tag/$TAG"
