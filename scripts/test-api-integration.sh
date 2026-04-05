#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ -f ".env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source ".env"
  set +a
fi

: "${DB_HOST:?DB_HOST must be set for integration tests}"
: "${DB_PORT:?DB_PORT must be set for integration tests}"
: "${DB_USERNAME:?DB_USERNAME must be set for integration tests}"
: "${DB_PASSWORD:?DB_PASSWORD must be set for integration tests}"

export AUTH_TOKEN_SECRET="${AUTH_TOKEN_SECRET:-integration-test-secret}"

cargo test -p api --test axum_routes "$@"
