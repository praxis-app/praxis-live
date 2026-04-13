# CLAUDE.md

This file provides guidance to [Claude Code](https://claude.com/product/claude-code) when working with code in this repository.

## Project overview

Praxis Live is a video calling app with collaborative decision-making (CDM) features. Users can have video calls while making group decisions through proposals, polls, and voting; all without breaking flow.

**Tech Stack**:

- Rust 1.93.0
- React/Vite
- TypeScript
- PostgreSQL
- WebSockets
- WebRTC

Refer to `README.md` for more information.

## Required verifications

- After code changes (not documentation-only), run `cargo fmt --check` and `cargo test` before signaling readiness.
- Fix failures locally and rerun the checks until clean.

## Git command restrictions

- Do not run git commands that stage, commit, amend, stash, or rewrite history (`git add`, `git commit`, `git reset`, etc.).
- Read-only inspection commands like `git status` or `git diff` are allowed when needed for context.

# Legacy code

Refer to `.tmp/praxis-chat` for the legacy codebase. There's a chance that .tmp is missing or empty. If so, run `git clone https://github.com/praxis-app/praxis-chat.git .tmp/praxis-chat` to get the legacy codebase.
