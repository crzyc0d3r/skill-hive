#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
log=$(mktemp)
set +e
cargo test -p hive-core --lib providers:: >"$log" 2>&1
providers=$?
cargo test -p hive-core --lib testing:: >>"$log" 2>&1
testing=$?
cargo test -p hive-cli --test contract >>"$log" 2>&1
cli=$?
set -e
code=0
if [ "$providers" -ne 0 ] || [ "$testing" -ne 0 ] || [ "$cli" -ne 0 ]; then
  code=1
fi
{
  echo "# n1 core"
  echo
  echo "hive-core providers exit ${providers}"
  echo "hive-core testing exit ${testing}"
  echo "hive-cli contract exit ${cli}"
  if [ "$code" -eq 0 ]; then
    echo "exit 0"
  else
    echo "exit ${code}"
  fi
  echo
  tail -n 80 "$log"
} >"${NODE_OUTPUT}"
rm -f "$log"
exit "$code"
