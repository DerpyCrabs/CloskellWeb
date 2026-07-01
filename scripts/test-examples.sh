#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

run_test() {
  project_name=$1
  project_dir=$2
  script_name=$3

  printf '\n==> %s: npm run %s\n' "$project_name" "$script_name"
  if (
    cd "$project_dir" || exit 1
    npm run "$script_name"
  ); then
    return 0
  else
    status=$?
    printf '\nExample test failed: %s %s exited with %s\n' "$project_name" "$script_name" "$status" >&2
    exit "$status"
  fi
}

run_test "hrweb" "$ROOT_DIR/projects/hrweb" "test:clsk"
run_test "hrweb" "$ROOT_DIR/projects/hrweb" "test:e2e"

run_test "better-swagger-ui" "$ROOT_DIR/projects/better-swagger-ui" "test:clsk"
run_test "better-swagger-ui" "$ROOT_DIR/projects/better-swagger-ui" "test:e2e"

run_test "derp-media-server" "$ROOT_DIR/projects/derp-media-server" "test:unit"
run_test "derp-media-server" "$ROOT_DIR/projects/derp-media-server" "test:e2e"

printf '\nAll example project tests passed.\n'
