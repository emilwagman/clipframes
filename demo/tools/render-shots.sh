#!/bin/bash
# Re-makes the website's figures from the real app, without showing anything on screen:
#   1. headless Chrome renders the Northwind demo page and the demo library's pictures,
#   2. Clipframes --render-shots draws the app's own views (bar, overlay, toast, library),
#   3. headless Chrome lays the two together into site/public/shots/*.jpg.
# Needs: a built app (app/build.sh), Node with playwright-core, and Google Chrome.
set -euo pipefail
cd "$(dirname "$0")/../.."
work=$(mktemp -d)
mkdir -p "$work/demo" && node demo/tools/northwind.mjs "$PWD/demo/northwind/index.html" "$work/demo"
app/build/Clipframes.app/Contents/MacOS/Clipframes --render-shots "$work/app" --demo "$work/demo"
mkdir -p "$work/out"
node demo/tools/compose.mjs "$work/demo" "$work/app" "$work/out"
cp "$work/out"/{element,screenshot,clip,bar,copied,library}.jpg site/public/shots/
echo "Updated site/public/shots/"
