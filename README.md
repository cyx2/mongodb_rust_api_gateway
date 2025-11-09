# Welcome to hello_rust

This application is a Rust-based API gateway that exposes MongoDB CRUD operations over HTTP. It is also a testbed for AI-driven engineering workflows, but end users can treat it as a regular service for forwarding REST calls to a single MongoDB cluster.

## Overview
- REST endpoints live under `/api/v1` and mirror MongoDB driver semantics (insert/find/update/replace/delete).
- Clients send JSON bodies containing `database`, `collection`, filters, updates, and optional driver-style options.
- Responses return driver-shaped payloads (`inserted_id`, `UpdateResult`, `DeleteResult`, etc.) plus clear HTTP status codes.

See `AGENTS.md` for the complete product specification and deeper engineering notes.

## Prerequisites
- Rust toolchain (`rustup`, `cargo`, Rust 1.75+ recommended).
- A reachable MongoDB cluster (local or remote) and credentials/URI.
- (Optional) `mongodb` CLI for verifying connectivity.

## Configuration
All settings are environment-driven. Copy the sample file and customize it for your cluster:
```bash
cp .env.example .env
```
Key variables:
- `MONGODB_URI`: Full connection string including credentials and options.
- `MONGODB_DEFAULT_DATABASE`, `MONGODB_DEFAULT_COLLECTION`: Defaults applied when requests omit them (if supported).
- `MONGODB_POOL_MIN_SIZE`, `MONGODB_POOL_MAX_SIZE`: Driver connection pooling.
- `MONGODB_CONNECT_TIMEOUT_MS`, `MONGODB_SERVER_SELECTION_TIMEOUT_MS`: Driver timeout knobs.
- `LOG_LEVEL`: `trace|debug|info|warn|error`.
- `APP_BIND_ADDRESS`: Address/port the HTTP server listens on (defaults to `127.0.0.1:3000`).

Optional knobs such as retry behavior or read preference can also be expressed via env vars (see `AGENTS.md`).

## Running the Gateway
1. Install dependencies and configure `.env` as above.
2. Start the service:
   ```bash
   cargo run
   ```
3. The server binds to `APP_BIND_ADDRESS`. Verify readiness via `curl http://127.0.0.1:3000/health` (or your configured port) once a health endpoint is implemented.

## API Quick Reference

All JSON requests **must** include `database` and `collection`. Optional `options` maps follow MongoDB driver naming, so fields such as `ordered`, `projection`, `sort`, `upsert`, and `array_filters` behave just like the Rust driver. Request bodies below illustrate the minimum payload necessary for the service to execute an operation.

### Insert
- `POST /api/v1/documents/insert-one`
  ```json
  {
    "database": "app",
    "collection": "users",
    "document": {
      "email": "quill@example.com",
      "name": "Quill"
    },
    "options": {
      "ordered": true
    }
  }
  ```
- `POST /api/v1/documents/insert-many`
  ```json
  {
    "database": "app",
    "collection": "users",
    "documents": [
      { "email": "rocket@example.com" },
      { "email": "groot@example.com" }
    ],
    "options": {
      "ordered": false
    }
  }
  ```

### Find
- `POST /api/v1/documents/find-one`
  ```json
  {
    "database": "app",
    "collection": "users",
    "filter": { "email": "quill@example.com" },
    "options": {
      "projection": { "_id": 0, "email": 1 }
    }
  }
  ```
- `POST /api/v1/documents/find-many`
  ```json
  {
    "database": "app",
    "collection": "users",
    "filter": { "team": "guardians" },
    "options": {
      "sort": { "created_at": -1 },
      "limit": 50,
      "skip": 0
    }
  }
  ```

### Update & Replace
- `POST /api/v1/documents/update-one`
  ```json
  {
    "database": "app",
    "collection": "users",
    "filter": { "email": "rocket@example.com" },
    "update": { "$set": { "nickname": "Rocket" } },
    "options": {
      "upsert": false
    }
  }
  ```
- `POST /api/v1/documents/update-many`
  ```json
  {
    "database": "app",
    "collection": "users",
    "filter": { "team": "guardians" },
    "update": { "$set": { "active": true } }
  }
  ```
- `POST /api/v1/documents/replace-one`
  ```json
  {
    "database": "app",
    "collection": "users",
    "filter": { "email": "groot@example.com" },
    "replacement": {
      "email": "groot@example.com",
      "name": "Groot",
      "language": "Flora colossi"
    },
    "options": {
      "upsert": true
    }
  }
  ```

### Delete
- `POST /api/v1/documents/delete-one`
  ```json
  {
    "database": "app",
    "collection": "users",
    "filter": { "email": "quill@example.com" }
  }
  ```
- `POST /api/v1/documents/delete-many`
  ```json
  {
    "database": "app",
    "collection": "users",
    "filter": { "team": "guardians" }
  }
  ```

### Collections Listing
- `GET /api/v1/collections?database=app`

Responses return MongoDB driver-shaped payloads (e.g., `{ "inserted_id": ... }`, driver `UpdateResult`, `DeleteResult`, or arrays of documents). See `AGENTS.md` for detailed error contracts and option support.

## Testing

The project includes comprehensive unit and integration tests.

### Running Tests

**Unit Tests** (no MongoDB required):
```bash
cargo test
```

**Integration Tests** (require MongoDB):
```bash
cargo test -- --ignored
```

**All Tests** (unit + integration):
```bash
cargo test -- --ignored
```

### Test Configuration

Integration tests use the `MONGODB_TEST_URI` environment variable (loaded from `.env` file):
- Default: `mongodb://localhost:27017`
- Can be set in `.env` file: `MONGODB_TEST_URI="mongodb+srv://..."`

### Test Options

**Run specific tests:**
```bash
cargo test test_name_pattern
cargo test --test integration_test -- --ignored test_name_pattern
```

**Show test output:**
```bash
cargo test -- --nocapture
```

**Run tests in parallel (default) or sequentially:**
```bash
cargo test -- --test-threads=1  # Sequential
```

**Run only ignored tests:**
```bash
cargo test -- --ignored
```

**Skip ignored tests (default):**
```bash
cargo test  # Only runs non-ignored tests
```

### Test Cleanup

After running integration tests, clean up test databases:
```bash
cargo test --test integration_test -- --ignored --nocapture cleanup::cleanup_test_databases
```

For detailed testing documentation, see [`tests/README.md`](tests/README.md).

## Troubleshooting
- Ensure MongoDB is reachable from the host running the gateway; driver errors surface as `502` responses with sanitized messages.
- Missing or invalid env vars cause a startup failure with actionable log output.
- If you encounter inconsistencies between this README and `AGENTS.md`, defer to `AGENTS.md` and open a PR to reconcile both.
