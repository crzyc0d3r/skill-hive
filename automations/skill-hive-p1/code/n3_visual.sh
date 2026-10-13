#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
log=$(mktemp)
set +e
npm run test:visual >"$log" 2>&1
code=$?
set -e
{
  echo "# n3 visual"
  echo
  echo "| playwright | ${code} |"
  echo
  tail -n 40 "$log"
} >"${NODE_OUTPUT}"
rm -f "$log"
exit "$code"
