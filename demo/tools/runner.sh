#!/bin/zsh
# Runs inside one Ghostty window on the recording Mac, so commands get Ghostty's
# Screen Recording permission. The hub sends shell lines through a private pipe:
#   ssh mac 'echo "screencapture -x /tmp/a.png" > /tmp/cf-runner/cmd'
# Each line runs in turn; when it finishes, /tmp/cf-runner/done holds its number.
dir=/tmp/cf-runner
rm -rf $dir; mkdir -m 700 $dir; mkfifo -m 600 $dir/cmd
n=0
print "Clipframes demo runner. Leave this window open; it can be minimized."
while true; do
  while IFS= read -r line; do
    n=$((n + 1))
    print "[$n] $line"
    eval "$line" >>$dir/log 2>&1
    print $n > $dir/done
  done < $dir/cmd
done
