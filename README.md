# Praxis Live - Chat-Based CDM with Video Calling

Praxis Live is a chat-based collaborative decision-making (CDM) app with first-class support for video calling. Groups can transition smoothly between messaging, live conversation, and structured decision-making without breaking flow or losing context.

Designed for organizations, teams, and communities that need robust group decision-making capabilities, it combines the familiarity of chat and video calls with flexible decision-making tools, multiple voting models, and forum-style organization when needed.

**Tech Stack**:

- Rust 1.93.0
- React/Vite
- TypeScript
- PostgreSQL
- WebSockets
- WebRTC

Praxis is free and open source software, as specified by the GNU General Public License.

## Work in progress

You are entering a construction yard. Things are going to change and break regularly as the project is still getting off the ground. Your feedback is highly welcome.

Please note that this is also an experimental approach within the Praxis project. The main repository is located at https://github.com/praxis-app/praxis.

## Integration tests

The Rust API route integration tests live in `api/tests/http_routes/` and use a real local Postgres server with temporary per-test databases.

Recommended commands:

- `cargo test -p api --test http_routes`
- `npm run test:api:integration`

The test support code already loads `.env` automatically. For database config, it accepts `PRAXIS_TEST_DATABASE_ADMIN_URL`, `DATABASE_URL`, or the `DB_HOST` / `DB_PORT` / `DB_USERNAME` / `DB_PASSWORD` variables.

These commands intentionally run only the `http_routes` integration-test target so the output stays focused on backend route coverage without the extra `running 0 tests` line from the empty unit-test target in `api/src/lib.rs`.
