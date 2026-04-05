#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  cat <<'EOF'
Run the Rust Axum integration-test target against a local Postgres server.

This intentionally runs only the `axum_routes` integration target so the
output stays focused on the backend route tests without the empty unit-test
target noise from `cargo test -p api`.

Usage:
  ./scripts/test-api-integration.sh
  ./scripts/test-api-integration.sh -- --nocapture
  ./scripts/test-api-integration.sh signup_returns_created_user_and_access_token -- --nocapture
EOF
  exit 0
fi

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
