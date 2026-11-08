#!/usr/bin/env bash
# Generate the HTML API reference of every quadra module into docs/api, one page per module.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
rm -rf docs/api
tools/v.sh doc -f html -m -o docs/api quadra
echo "API docs: docs/api/ (one page per module, e.g. docs/api/quadra.core.html)"
