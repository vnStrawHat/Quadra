#!/usr/bin/env bash
# Waits for the macOS job of the branch's newest commit and prints what it said (specs/23):
# tools/ci_mac.sh [branch]. The job's log needs rights to the repository; what it says as
# annotations of the run (what passed, the end of what failed) is public, and read here.
set -euo pipefail
cd "$(dirname "$0")/.."
api=https://api.github.com/repos/vnStrawHat/Quadra
branch="${1:-macos}"
run=$(curl -s "$api/actions/runs?branch=$branch&per_page=1" |
	python -c "import json,sys; r=json.load(sys.stdin)['workflow_runs'][0]; print(r['id'])")
echo "run $run: https://github.com/vnStrawHat/Quadra/actions/runs/$run"
until state=$(curl -s "$api/actions/runs/$run" |
	python -c "import json,sys; d=json.load(sys.stdin); print(d.get('status'), d.get('conclusion'), d.get('head_sha','')[:7])") &&
	[ "${state%% *}" = completed ]; do
	sleep 45
done
echo "$state"
curl -s "$api/actions/runs/$run/jobs" | PYTHONUTF8=1 python -c "
import json, sys, urllib.request
for job in json.load(sys.stdin).get('jobs', []):
    for step in job['steps']:
        if step['conclusion'] not in ('success', 'skipped'):
            print('step:', step['name'], step['conclusion'])
    said = json.load(urllib.request.urlopen('$api/check-runs/%d/annotations' % job['id']))
    for note in said:
        if note.get('title'):
            print('--', note.get('annotation_level'), '|', note.get('title'))
            print((note.get('message') or '')[-6000:])
"
