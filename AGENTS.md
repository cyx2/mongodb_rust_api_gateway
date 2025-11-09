# Agent Handbook

## Repo Purpose
`hello_rust` is a ground-up Rust API gateway that fronts MongoDB CRUD operations. The codebase doubles as a testbed for agentic software workflows: nearly every change is produced by AI coding tools (OpenAI Codex CLI/web plus GitHub Copilot). Keep docs and automation friendly for autonomous contributors.

> **Sync Note:** README.md targets end users operating the gateway; AGENTS.md is the developer/agent playbook. When overlapping topics appear (e.g., env vars, API surface), keep both files consistent and update them together.

## Product Specification

### Overview
Expose REST endpoints that mirror the MongoDB Rust driver’s CRUD surface. Clients submit JSON payloads describing the database, collection, filters, and options; the service forwards them to a single configured MongoDB cluster and returns structured responses/errors. All connectivity is supplied via environment variables.

### Core Behavior
- Use the official MongoDB Rust driver for all CRUD paths (single/multi inserts, finds, updates, replacements, deletes). Defer aggregations, transactions, bulk writes, and change streams.
- Keep endpoint semantics, payload shapes, and option names 1:1 with the driver.
- Require `database` and `collection` per request to scope operations explicitly.
- Skip auth for MVP (no API keys or sessions).
- Accept ergonomic JSON payloads (BSON-like filters, option maps).
- Emit precise HTTP status codes plus machine-readable error payloads to separate validation failures from driver issues.

### CRUD Endpoints (`/api/v1`, `Content-Type: application/json`)
- `POST /api/v1/documents/insert-one`: `{ database, collection, document, options? }` → `{ inserted_id }`.
- `POST /api/v1/documents/insert-many`: `{ database, collection, documents[], options? }` → `{ inserted_ids[] }`.
- `POST /api/v1/documents/find-one`: `{ database, collection, filter?, options? }` → `{ document }` or `404`.
- `POST /api/v1/documents/find-many`: `{ database, collection, filter?, options { projection?, sort?, limit?, skip? } }` → `{ documents[] }`.
- `POST /api/v1/documents/update-one|update-many`: `{ database, collection, filter, update, options? }` → driver `UpdateResult`.
- `POST /api/v1/documents/replace-one`: `{ database, collection, filter, replacement, options? }` → driver `UpdateResult`.
- `POST /api/v1/documents/delete-one|delete-many`: `{ database, collection, filter, options? }` → driver `DeleteResult`.
- `GET /api/v1/collections?database=app`: list collections for discovery.

Example request bodies for each operation live in `README.md` under **API Quick Reference**; keep those samples aligned with the shapes described above when endpoints evolve.

### Error Contract
- `400` with `{ "error": "validation_error", "details": "..." }` for malformed payloads or missing identifiers.
- `404` when single-document operations find no match.
- `502` for MongoDB driver/network failures; include sanitized driver message + correlation ID.
- `500` for unexpected errors; always include a correlation ID.

## Technical Requirements

### Configuration & Environment
- All runtime knobs come from env vars; no hard-coded defaults.
- Load `.env` on startup (e.g., `dotenvy::dotenv().ok();`).
- `.env.example` documents required keys:
  - Connection: `MONGODB_URI`, `MONGODB_DEFAULT_DATABASE`, `MONGODB_DEFAULT_COLLECTION`
  - Pool sizing: `MONGODB_POOL_MIN_SIZE`, `MONGODB_POOL_MAX_SIZE`
  - Timeouts: `MONGODB_CONNECT_TIMEOUT_MS`, `MONGODB_SERVER_SELECTION_TIMEOUT_MS`
  - Logging/binding: `LOG_LEVEL`, `APP_BIND_ADDRESS`
  - Optional: retry knobs, read preference, etc.

### Startup Expectations
- Load env, then initialize logging/config.
- Validate mandatory env vars; fail fast with actionable errors.
- Single-cluster assumption: restart the process when changing cluster settings.

## Developer Workflow

### Local Setup
1. Install Rust toolchain (`rustup`, `cargo`) and ensure MongoDB is reachable for integration tests.
2. Copy `cp .env.example .env` and customize connection/pool/timeout values.
3. Run `cargo run` (binds to `APP_BIND_ADDRESS`, default `127.0.0.1:3000`).

### Required Commands
- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test`
Run all three before submitting a change; release builds use `cargo build --release` as needed.

### VS Code Debugging
`.vscode/launch.json` provides a “Debug Mongo Gateway” configuration that executes `cargo run` with env loading.

## Engineering Conventions

### Project Structure & Modules
Crate scaffolded via `cargo init`; keep sources in `src/`, extra binaries under `src/bin/`, integration tests in `tests/`, and optional samples in `examples/`/`benches/`. Ignore `target/`. Group modules by domain, re-export shared types from `lib.rs`.

### Build/Test Commands
- `cargo build` / `cargo build --release`
- `cargo run -- [args]`
- `cargo fmt`, `cargo clippy -- -D warnings`
- `cargo doc --open`

### Coding Style
Use Rust defaults (4 spaces, snake_case functions/modules, CamelCase types, SCREAMING_SNAKE_CASE consts). Split files when >300 lines or multi-concern. Avoid `todo!()` in merged code. Derive `Debug`, `Clone`, `PartialEq` on domain structs when useful for tests.

### Testing Guidelines

**Test Structure:**
- Unit tests: Inline tests under `#[cfg(test)]` in each module
- Integration tests: Separate test files in `tests/` directory
- Test helpers: Shared utilities in `tests/common/mod.rs`

**Test Naming:**
- Name test functions after behaviors (e.g., `serializes_minimal_header`, `rejects_invalid_u32_values`)
- Use descriptive names that explain what is being tested

**Running Tests:**

**Unit Tests** (no external dependencies):
```bash
cargo test                    # Run all unit tests
cargo test --lib              # Run only library unit tests
cargo test module_name::tests # Run tests in a specific module
```

**Integration Tests** (require MongoDB):
```bash
cargo test -- --ignored                              # Run all integration tests
cargo test --test integration_test -- --ignored     # Run integration test suite
cargo test --test integration_test -- --ignored test_name  # Run specific test
```

**Test Options:**
```bash
cargo test -- --nocapture          # Show stdout/stderr output
cargo test -- --test-threads=1     # Run tests sequentially
cargo test -- --ignored            # Include tests marked with #[ignore]
cargo test -- --no-fail-fast       # Continue running after failures
cargo test -- --quiet              # Minimal output (one char per test)
```

**Test Configuration:**
- Integration tests use `MONGODB_TEST_URI` from `.env` file (defaults to `mongodb://localhost:27017`)
- Test databases follow pattern `test_db_*` and can be cleaned up after runs
- See `tests/README.md` for detailed integration test setup and MongoDB configuration

**Pre-commit Checklist:**
- Run `cargo fmt` to ensure formatting
- Run `cargo clippy -- -D warnings` to catch linting issues
- Run `cargo test` to verify unit tests pass
- Run `cargo test -- --ignored` to verify integration tests pass (if MongoDB available)
- Run cleanup: `cargo test --test integration_test -- --ignored cleanup::cleanup_test_databases`

### Commit & PR Habits
Concise present-tense summaries (`init: scaffold cargo crate`), reference issues when possible (`fix: handle empty payload (#42)`). Each PR should outline intent, testing proof (`cargo test`, `cargo fmt`), and follow-up work. Attach screenshots/CLI transcripts for user-facing changes. Keep PRs focused and rebase onto `main` before requesting review.
