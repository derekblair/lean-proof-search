#!/bin/sh
# Fetch an immutable core revision without changing this lane's source tree.
set -eu
export PATH=/root/.cargo/bin:/root/.elan/bin:$PATH
cd "$(dirname "$0")/.."
revision=$(cat bench/core-revision.txt)
if [ ! -e bench/core-checkout ]; then
    git fetch origin "$revision"
    git worktree add --detach bench/core-checkout "$revision"
fi
actual=$(git -C bench/core-checkout rev-parse HEAD)
if [ "$actual" != "$revision" ] || [ -n "$(git -C bench/core-checkout status --porcelain)" ]; then
    echo 'Core checkout does not match the clean pinned revision.' >&2
    exit 1
fi
