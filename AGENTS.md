# AGENTS.md

This file provides guidance to the agents when working with code in this repository.

## Project overview

Praxis Live is a video calling app with collaborative decision-making (CDM) features. Users can have video calls while making group decisions through proposals, polls, and voting; all without breaking flow.

**Tech Stack**:

- Rust 1.97.0
- React/Vite
- TypeScript
- PostgreSQL
- WebSockets
- WebRTC

Refer to `README.md` for more information.

## Frontend architecture

- Prefer one React component per file. When a feature needs multiple related components, place them in a dedicated folder and split each component into its own file, with shared non-component helpers in separate utility files.

## Backend architecture

The Rust backend under `api/src` is organized into modules divided by domain or feature, such as calls, messages, polls, and votes. When adding backend functionality, follow the structure used by the existing modules:

- `service.rs` contains the business logic for the module.
- `handlers.rs` contains thin route handlers. Handlers should call service functions and handle request lifecycle concerns such as extracting inputs, shaping responses, mapping errors, and websocket interactions; keep business logic out of handlers whenever possible.
- `routes.rs` registers endpoints and wires them to handler functions from `handlers.rs`.
- `mod.rs` declares the module and its internal files.
- `types.rs` contains module-specific request/response and data transfer types.
- `models.rs`, when present, contains persisted/domain model definitions.
- `extractors.rs`, when present, contains custom request extraction logic.

## Required verifications

- After code changes (not documentation-only), run `cargo fmt --check` and `cargo test` before signaling readiness.
- After large code changes, run `npm run test:e2e` to verify that the changes work as expected.
- Fix failures locally and rerun the checks until clean.

## Git command restrictions

- Do not run git commands that stage, commit, amend, stash, or rewrite history (`git add`, `git commit`, `git reset`, etc.).
- Read-only inspection commands like `git status` or `git diff` are allowed when needed for context.

# Legacy code

Refer to `.tmp/praxis-chat` for the legacy codebase. There's a chance that .tmp is missing or empty. If so, run `git clone https://github.com/praxis-app/praxis-chat.git .tmp/praxis-chat` to get the legacy codebase.
