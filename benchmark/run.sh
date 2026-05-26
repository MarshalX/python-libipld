#!/usr/bin/env bash
# Usage: ./run.sh           # runs encode + decode
#        ./run.sh encode    # runs encode only
#        ./run.sh decode    # runs decode only

set -euo pipefail

cd "$(dirname "$0")"

TARGET="${1:-}"
if [[ -n "$TARGET" ]]; then
    TEST_PATH="test_${TARGET}.py"
else
    TEST_PATH=""
fi

uv run --no-project --with-requirements requirements.txt \
    pytest \
        --verbose \
        --benchmark-enable \
        --benchmark-min-time=1 \
        --benchmark-max-time=5 \
        --benchmark-disable-gc \
        --benchmark-autosave \
        --benchmark-save-data \
        --benchmark-json=results.json \
        --random-order \
        ${TEST_PATH}

uv run --no-project --with-requirements requirements.txt python chart.py results.json
