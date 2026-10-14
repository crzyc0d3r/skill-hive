#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
log=$(mktemp)
set +e
cargo test --workspace >"$log" 2>&1
code=$?
set -e
{
  echo "# n1 rust"
  echo
  echo "exit ${code}"
  tail -n 30 "$log"
} >"${NODE_OUTPUT}"
rm -f "$log"
exit "$code"
