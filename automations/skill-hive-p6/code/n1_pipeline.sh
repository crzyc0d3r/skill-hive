#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
log=$(mktemp)
set +e
node --test scripts/release.test.mjs >"$log" 2>&1
release=$?
node scripts/check-hardening.mjs >>"$log" 2>&1
hardening=$?
node scripts/check-distribution.mjs >>"$log" 2>&1
distribution=$?
node scripts/prepare-release.mjs >>"$log" 2>&1
prepare=$?
set -e
code=0
if [ "$release" -ne 0 ] || [ "$hardening" -ne 0 ] || [ "$distribution" -ne 0 ] || [ "$prepare" -ne 0 ]; then
  code=1
fi
{
  echo "# n1 pipeline"
  echo
  echo "node release tests exit ${release}"
  echo "check-hardening exit ${hardening}"
  echo "check-distribution exit ${distribution}"
  echo "prepare-release exit ${prepare}"
  if [ "$code" -eq 0 ]; then
    echo "exit 0"
  else
    echo "exit ${code}"
  fi
  echo
  tail -n 60 "$log"
} >"${NODE_OUTPUT}"
rm -f "$log"
exit "$code"
