#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
log=$(mktemp)
set +e
cargo test -p hive-core --lib generation:: >"$log" 2>&1
generation=$?
cargo test -p hive-core --lib analytics:: >>"$log" 2>&1
analytics=$?
cargo test -p hive-cli --test contract >>"$log" 2>&1
cli=$?
set -e
code=0
if [ "$generation" -ne 0 ] || [ "$analytics" -ne 0 ] || [ "$cli" -ne 0 ]; then
  code=1
fi
{
  echo "# n1 core"
  echo
  echo "hive-core generation exit ${generation}"
  echo "hive-core analytics exit ${analytics}"
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
