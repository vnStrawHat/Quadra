#!/usr/bin/env bash
# Waits for the macOS job of the branch's commit (pushed) and says what it did (specs/23):
# tools/ci_mac.sh [branch]. It asks GitHub through `gh`, signed in (`gh auth login`): the
# run's notice (the tests that passed), the end of the log of what failed, and the pictures
# the job drew on the Mac, which it puts in .tmp/ci/mac-dark: the gallery's pages in its
# folder `dark`, Markdown's screens in `markdown`, the runner's screen with Markdown in a
# real window in `window` (tools/window_mac.sh).
set -euo pipefail
cd "$(dirname "$0")/.."
# Where the installer of Windows puts it, for a shell opened before it was installed.
gh=$(command -v gh || echo "/c/Program Files/GitHub CLI/gh.exe")
branch="${1:-macos}"
want=$(git rev-parse "$branch")
# The run appears a while after the push.
until run=$("$gh" run list --workflow macOS --commit "$want" --limit 1 --json databaseId \
	--jq '.[0].databaseId') && [ -n "$run" ]; do
	sleep 20
done
echo "run $run: https://github.com/vnStrawHat/Quadra/actions/runs/$run"
"$gh" run watch "$run" --interval 30 > /dev/null 2>&1 || true
"$gh" run view "$run" --json conclusion,headSha --jq '"\(.conclusion) \(.headSha[0:7])"'
job=$("$gh" run view "$run" --json jobs --jq '.jobs[0].databaseId')
"$gh" api "repos/{owner}/{repo}/check-runs/$job/annotations" \
	--jq '.[] | select(.title == "Tests on a Mac") | .message' || true
# What failed: the log's lines less the job's and the step's names, and less what cargo
# says of every crate.
"$gh" run view "$run" --log-failed 2> /dev/null | cut -f3- |
	grep -vE ' +(Compiling|Checking|Downloading|Downloaded|Fresh) ' | tail -n 150 || true
rm -rf .tmp/ci/mac-dark
if "$gh" run download "$run" -n gallery-dark-macos -D .tmp/ci/mac-dark 2> /dev/null; then
	echo "pictures: .tmp/ci/mac-dark ($(find .tmp/ci/mac-dark -name '*.png' | wc -l) of them)"
else
	echo "pictures: none"
fi
