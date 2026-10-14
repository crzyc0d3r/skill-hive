#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
log=$(mktemp)
set +e
cargo test -p hive-core --lib updater:: >"$log" 2>&1
updater=$?
cargo test -p hive-core --lib sanitize_rejects_traversal_payloads >>"$log" 2>&1
guard=$?
cargo test -p hive-core --lib redacts_provider_tokens_and_bearer >>"$log" 2>&1
scrub=$?
npx vitest run src/test/phase6.test.tsx src/test/shell.test.tsx >>"$log" 2>&1
vitest=$?
set -e
code=0
if [ "$updater" -ne 0 ] || [ "$guard" -ne 0 ] || [ "$scrub" -ne 0 ] || [ "$vitest" -ne 0 ]; then
  code=1
fi
{
  echo "# n2 updater"
  echo
  echo "hive-core updater exit ${updater}"
  echo "path guard exit ${guard}"
  echo "log scrub exit ${scrub}"
  echo "vitest exit ${vitest}"
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
