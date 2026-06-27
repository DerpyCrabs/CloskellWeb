#!/usr/bin/env sh
set -u

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
FAILED=0
FAILURES=""

run_test() {
  project_name=$1
  project_dir=$2
  script_name=$3

  printf '\n==> %s: npm run %s\n' "$project_name" "$script_name"
  (
    cd "$project_dir" || exit 1
    npm run "$script_name"
  )
  status=$?

  if [ "$status" -ne 0 ]; then
    FAILED=1
    FAILURES="${FAILURES}
- ${project_name} ${script_name}: exit code ${status}"
  fi
}

run_test "hrweb" "$ROOT_DIR/projects/hrweb" "test:clsk"
run_test "hrweb" "$ROOT_DIR/projects/hrweb" "test:e2e"

run_test "better-swagger-ui" "$ROOT_DIR/projects/better-swagger-ui" "test:clsk"
run_test "better-swagger-ui" "$ROOT_DIR/projects/better-swagger-ui" "test:e2e"

if [ "$FAILED" -ne 0 ]; then
  printf '\nExample test failures:%s\n' "$FAILURES" >&2
  exit 1
fi

printf '\nAll example project tests passed.\n'
