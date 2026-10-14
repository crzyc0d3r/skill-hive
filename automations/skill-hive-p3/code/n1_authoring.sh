#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
log=$(mktemp)
set +e
cargo test -p hive-core --lib authoring -- --test-threads=8 >"$log" 2>&1
core=$?
cargo test -p hive-cli --test contract >>"$log" 2>&1
cli=$?
set -e
code=0
if [ "$core" -ne 0 ] || [ "$cli" -ne 0 ]; then
  code=1
fi
{
  echo "# n1 authoring"
  echo
  echo "hive-core authoring exit ${core}"
  echo "hive-cli contract exit ${cli}"
  if [ "$code" -eq 0 ]; then
    echo "exit 0"
  else
    echo "exit ${code}"
  fi
  echo
  tail -n 40 "$log"
} >"${NODE_OUTPUT}"
rm -f "$log"
exit "$code"
