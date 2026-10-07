#!/bin/bash
# clipframes: turn a screen recording into evenly spaced frames and copy a
# one-line reference to the clipboard, ready to paste into Claude Code or Codex.
#
#   clipframes.sh             process new "Screen Recording*.mov" on the Desktop
#   clipframes.sh FILE.mov    process one video (any path; the original is copied)
#
# Settings (env): CLIPFRAMES_FPS (default 2), CLIPFRAMES_MAX (default 20),
# CLIPFRAMES_WIDTH (default 1280), CLIPFRAMES_OUT (default ~/Clips).
set -euo pipefail
export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin"

FPS="${CLIPFRAMES_FPS:-2}"
MAX="${CLIPFRAMES_MAX:-20}"
WIDTH="${CLIPFRAMES_WIDTH:-1280}"
OUT="${CLIPFRAMES_OUT:-$HOME/Clips}"
WATCH="$HOME/Desktop"
LOG="$OUT/.clipframes.log"
mkdir -p "$OUT"

log() { echo "$(date '+%F %T') $*" >> "$LOG"; }
notify() { osascript -e "display notification \"$1\" with title \"Clip ready\"" >/dev/null 2>&1 || true; }

# Wait until the file stops growing (the recorder may still be writing).
wait_stable() {
  local f="$1" a b
  for _ in $(seq 1 30); do
    a=$(stat -f %z "$f" 2>/dev/null || echo 0)
    sleep 1
    b=$(stat -f %z "$f" 2>/dev/null || echo 0)
    [[ "$a" == "$b" && "$b" != 0 ]] && return 0
  done
  return 1
}

process() {
  local src="$1" move="$2"
  wait_stable "$src" || { log "gave up waiting on $src"; return 1; }

  local stamp dir dur n interval
  stamp=$(date -r "$(stat -f %m "$src")" '+%Y-%m-%d_%H-%M-%S')
  dir="$OUT/$stamp"
  [[ -e "$dir" ]] && dir="${dir}_$$"
  mkdir -p "$dir/frames"

  if [[ "$move" == 1 ]]; then mv "$src" "$dir/video.mov"; else cp "$src" "$dir/video.${src##*.}"; fi
  local video
  video=$(ls "$dir"/video.* | head -1)

  dur=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$video")
  # Frame count: FPS per second, capped at MAX, at least 1. Spacing stays even.
  n=$(awk -v d="$dur" -v f="$FPS" -v m="$MAX" 'BEGIN{n=int(d*f+0.5); if(n>m)n=m; if(n<1)n=1; print n}')
  interval=$(awk -v d="$dur" -v n="$n" 'BEGIN{printf "%.2f", d/n}')

  ffmpeg -v error -i "$video" \
    -vf "fps=${n}/${dur},scale='min(${WIDTH},iw)':-2" \
    -frames:v "$n" "$dir/frames/%03d.png"

  local count secs line
  count=$(ls "$dir/frames" | wc -l | tr -d ' ')
  secs=$(awk -v d="$dur" 'BEGIN{printf "%.1f", d}')
  line="[Screen recording, ${secs}s: ${count} frames ${interval}s apart, view in order: $dir/frames/]"
  printf '%s' "$line" | pbcopy
  printf '%s\n' "$line" > "$dir/prompt.txt"
  log "ok $dir ($count frames)"
  notify "${count} frames, ${secs}s. Reference copied."
  echo "$line"
}

if [[ $# -gt 0 ]]; then
  for f in "$@"; do process "$f" 0; done
  exit 0
fi

# Watcher mode: only recordings from the last 10 minutes, so old ones stay put.
shopt -s nullglob
for f in "$WATCH"/Screen\ Recording*.mov; do
  if [[ $(( $(date +%s) - $(stat -f %m "$f") )) -lt 600 ]]; then
    process "$f" 1 || true
  fi
done
