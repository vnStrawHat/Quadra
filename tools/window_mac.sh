#!/usr/bin/env bash
# tools/window_mac.sh: on a Mac with a screen (the CI's runner has one), Markdown from its
# bundle in a real window, as the screen shows it (specs/23, "What a real Mac is asked"):
# the system's three buttons over the app's title bar, which no picture drawn without a
# window has. The app is opened as the Finder opens a file, then asked for a second file
# while it runs, then asked to quit as the menu bar's Quit asks. The pictures of the whole
# screen go to .tmp/shots/window; what the script saw is said in lines that start with
# `window_mac:`. It fails only when there is no bundle to open: a screen that is not
# captured, or an app that does not answer, is said and no more.
set -uo pipefail
cd "$(dirname "$0")/.."
[ "$(uname)" = Darwin ] || { echo "window_mac: a Mac shows a window" >&2; exit 1; }
app="$PWD/target/package/Markdown.app"
[ -d "$app" ] || PACKAGE_PROFILE="${PACKAGE_PROFILE:-dev}" bash tools/package_mac.sh markdown || exit 1
out=.tmp/shots/window
rm -rf "$out"
mkdir -p "$out"

# A command given so many seconds: one that waits for a question of the system's, which
# nobody answers here, is ended.
within() { perl -e 'alarm shift; exec @ARGV' "$@"; }
shot() {
	sleep "$1"
	if within 20 screencapture -x "$out/$2.png" && [ -s "$out/$2.png" ]; then
		echo "window_mac: $out/$2.png"
	else
		echo "window_mac: the screen was not captured ($2)"
	fi
}
running() { pgrep -x markdown > /dev/null; }

# As the Finder opens a file: the system starts the app and asks it for the file.
open -a "$app" "$PWD/bench/markdown-shots/src/sample.md" || echo "window_mac: open failed"
shot 12 opened
running && echo "window_mac: the app runs" || echo "window_mac: the app does not run"
# A second file with the app running: no second process, a tab.
open -a "$app" "$PWD/bench/markdown-shots/src/tables.md" || echo "window_mac: the second open failed"
shot 5 second
echo "window_mac: $(pgrep -x markdown | wc -l | tr -d ' ') process(es) of the app"
# Quit, as the menu bar's asks: the windows are asked to close, and the app ends with none.
within 15 osascript -e 'tell application "Markdown" to quit' > /dev/null 2>&1
sleep 4
if running; then
	echo "window_mac: Quit did not end the app"
	pkill -x markdown
else
	echo "window_mac: Quit ended the app"
fi
exit 0
