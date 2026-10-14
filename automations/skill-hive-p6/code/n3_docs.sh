#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
log=$(mktemp)
set +e
node scripts/check-docs.mjs >"$log" 2>&1
docs=$?
node scripts/check-i18n.mjs >>"$log" 2>&1
i18n=$?
set -e
code=0
if [ "$docs" -ne 0 ] || [ "$i18n" -ne 0 ]; then
  code=1
fi
{
  echo "# n3 docs"
  echo
  echo "check-docs exit ${docs}"
  echo "check-i18n exit ${i18n}"
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
