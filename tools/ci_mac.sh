#!/usr/bin/env bash
# Waits for the macOS job of the branch's commit (pushed) and prints what it said (specs/23):
# tools/ci_mac.sh [branch]. The job's log needs rights to the repository; what it says as
# annotations of the run (what passed, the end of what failed) is public, and read here.
set -euo pipefail
cd "$(dirname "$0")/.."
api=https://api.github.com/repos/vnStrawHat/Quadra
branch="${1:-macos}"
# The run of the branch's commit as it is here: it appears a while after the push. Few
# requests: GitHub answers sixty an hour to who has no token.
want=$(git rev-parse "$branch")
until run=$(curl -s "$api/actions/runs?branch=$branch&per_page=5" | python -c "
import json, sys
print(next(r['id'] for r in json.load(sys.stdin).get('workflow_runs', []) if r['head_sha'] == '$want'))
" 2>/dev/null); do
	sleep 30
done
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
