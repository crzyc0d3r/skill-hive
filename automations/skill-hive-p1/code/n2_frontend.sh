#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
rows=$(mktemp)
err=$(mktemp)
fail=0
run() {
  local name="$1"
  shift
  local log
  log=$(mktemp)
  set +e
  "$@" >"$log" 2>&1
  local code=$?
  set -e
  echo "| ${name} | ${code} |" >>"$rows"
  if [ "$code" -ne 0 ]; then
    fail=$code
    echo "## ${name}" >>"$err"
    tail -n 40 "$log" >>"$err"
  fi
  rm -f "$log"
}
run "npm test" npm test
run "npm run lint" npm run lint
run "npx tsc -b" npx tsc -b --pretty false
run "check:i18n" npm run check:i18n
run "check:headers" npm run check:headers
{
  echo "# n2 frontend"
  echo
  cat "$rows"
  echo
  cat "$err"
} >"${NODE_OUTPUT}"
rm -f "$rows" "$err"
exit "$fail"
