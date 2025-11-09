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

### Files to Ignore
- `WORKLOG.md`: Human-written worklog documenting the development experience. AI agents should not modify this file.
- `target/`: Build artifacts (already in `.gitignore`).
- `.env`: Local environment configuration (already in `.gitignore`).

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

**All Tests** (unit + integration, recommended):
```bash
cargo test                    # Run all tests (integration tests skip if MongoDB unavailable)
```

**Unit Tests Only** (no external dependencies):
```bash
cargo test --lib              # Run only library unit tests
cargo test module_name::tests # Run tests in a specific module
```

**Integration Tests Only** (require MongoDB):
```bash
cargo test --tests                              # Run all integration tests
cargo test --test integration_test              # Run integration test suite
cargo test --test integration_test test_name    # Run specific integration test
```

**Test Options:**
```bash
cargo test -- --nocapture          # Show stdout/stderr output
cargo test -- --test-threads=1     # Run tests sequentially (ensures cleanup runs last)
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
- Run `cargo test` to verify all tests pass (integration tests skip if MongoDB unavailable)
- Run `cargo test --tests` to verify integration tests pass (if MongoDB available)
- Run cleanup: `cargo test --test integration_test zzz_cleanup_test_databases`
- Run all tests with cleanup last: `cargo test --test-threads=1`

### Commit & PR Habits
Concise present-tense summaries (`init: scaffold cargo crate`), reference issues when possible (`fix: handle empty payload (#42)`). Each PR should outline intent, testing proof (`cargo test`, `cargo fmt`), and follow-up work. Attach screenshots/CLI transcripts for user-facing changes. Keep PRs focused and rebase onto `main` before requesting review.

## Code Quality Standards

### Error Handling
- Use `ApiError` from `src/error.rs` for all HTTP-facing errors
- Map MongoDB driver errors through `map_driver_error` to sanitize sensitive information
- Always include correlation IDs in error responses for traceability
- Return appropriate HTTP status codes (400 for validation, 404 for not found, 502 for MongoDB errors, 500 for unexpected errors)

### Logging
- Use structured logging via `tracing` crate
- Log requests at `info` level using `log_request_received`, `log_request_success`, `log_request_failure` helpers
- Include endpoint context in all log messages
- Never log sensitive data (passwords, tokens, full connection strings)
- See `docs/request_logging.md` for detailed logging architecture

### Code Organization
- Keep modules focused on a single responsibility
- Split files when they exceed ~300 lines or handle multiple concerns
- Re-export public types from `lib.rs` for external consumers
- Use `#[cfg(test)]` modules for unit tests within source files

### Dependencies
- Prefer standard library solutions when possible
- Use well-maintained crates from the Rust ecosystem
- Document non-obvious dependency choices in code comments
- Keep `Cargo.toml` dependencies up to date and minimal

## Architecture Decisions

### Why Axum?
- Modern async web framework built on Tokio
- Excellent performance and ergonomics
- Strong type safety with extractors
- Active development and community support

### Why MongoDB Rust Driver?
- Official driver maintained by MongoDB
- Full feature parity with other MongoDB drivers
- Strong type safety with BSON types
- Comprehensive documentation and examples

### Why Environment Variables?
- No hard-coded configuration values
- Easy deployment across environments
- Standard practice for containerized applications
- Clear separation of code and configuration

### Why Structured Logging?
- Better observability in production
- Easier debugging with correlation IDs
- Integration with log aggregation systems
- Consistent log format across all endpoints

## Extension Points

### Adding New Endpoints
1. Define request/response types in `src/models.rs` (derive `Debug`, `Serialize`, `Deserialize`)
2. Add handler function in `src/routes.rs` following existing patterns
3. Use logging helpers (`log_request_received`, `log_request_success`, `log_request_failure`)
4. Register route in `router()` function
5. Add integration tests in `tests/integration_test.rs`
6. Update `README.md` and `AGENTS.md` with endpoint documentation

### Adding New Configuration Options
1. Add field to `Config` struct in `src/config.rs`
2. Parse from environment variable in `Config::from_env()`
3. Document in `.env.example` with comments
4. Update `AGENTS.md` Technical Requirements section
5. Update `README.md` Configuration section if user-facing

### Adding New Error Types
1. Add variant to `ApiError` enum in `src/error.rs`
2. Implement `IntoResponse` if needed for custom HTTP status/body
3. Ensure `status()` method returns correct HTTP code
4. Add unit tests for error serialization
5. Document in error contract section of `AGENTS.md`

## Performance Optimization

### Connection Pooling
- Configure `MONGODB_POOL_MIN_SIZE` and `MONGODB_POOL_MAX_SIZE` based on expected load
- Default values are reasonable for most use cases
- Monitor connection pool metrics in production

### Request Handling
- All handlers are async and non-blocking
- MongoDB operations are async and use connection pooling
- No blocking I/O in request handlers
- Consider adding request timeout middleware for production

### Logging Overhead
- Use appropriate `LOG_LEVEL` in production (`warn` or `error`)
- Structured logging has minimal overhead
- Avoid logging large payloads in production

## Security Considerations

### Current Limitations
- No authentication or authorization
- No rate limiting
- No input sanitization beyond basic validation
- Direct MongoDB access without query validation

### Production Hardening Checklist
- [ ] Add authentication (API keys, OAuth, etc.)
- [ ] Implement rate limiting
- [ ] Add request size limits
- [ ] Validate MongoDB query structures
- [ ] Use TLS for MongoDB connections
- [ ] Add reverse proxy with SSL termination
- [ ] Implement CORS policies if needed
- [ ] Add request/response logging filtering for sensitive data
- [ ] Set up monitoring and alerting
- [ ] Regular security audits

## Monitoring & Observability

### Logs
- Structured JSON logs via `tracing` subscriber
- Request/response logging at `info` level
- Error logging at `warn` level with correlation IDs
- Configurable log level via `LOG_LEVEL` env var

### Metrics (Future)
- Consider adding Prometheus metrics endpoint
- Track request counts, latencies, error rates
- Monitor MongoDB connection pool usage
- Track endpoint-specific metrics

### Health Checks (Future)
- Add `/health` endpoint for load balancer health checks
- Check MongoDB connectivity in health endpoint
- Return appropriate status codes based on service state

## Deployment Considerations

### Environment Setup
- All configuration via environment variables
- No hard-coded values in code
- `.env.example` documents all required variables
- Fail fast on missing required configuration

### Containerization
- Dockerfile should set environment variables
- Use multi-stage builds for smaller images
- Consider using distroless base images for security
- Document required environment variables in container docs

### Scaling
- Stateless design allows horizontal scaling
- Each instance maintains its own MongoDB connection pool
- Consider connection pool sizing when scaling horizontally
- Load balancer should distribute requests evenly
