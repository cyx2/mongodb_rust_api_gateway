# hello_rust

A production-ready Rust API gateway that exposes MongoDB CRUD operations over HTTP. This service provides a RESTful interface to MongoDB, allowing clients to perform database operations without direct MongoDB driver dependencies.

## Quick Start

```bash
# 1. Clone and navigate to the project
cd hello_rust

# 2. Copy environment template
cp .env.example .env

# 3. Edit .env with your MongoDB connection string
# Set MONGODB_URI="mongodb://localhost:27017" (or your MongoDB URI)

# 4. Build and run
cargo run

# 5. Test the API (in another terminal)
curl -X POST http://127.0.0.1:3000/api/v1/documents/insert-one \
  -H "Content-Type: application/json" \
  -d '{"database":"test","collection":"users","document":{"name":"Alice"}}'
```

## Overview

**Key Features:**
- RESTful API endpoints under `/api/v1` that mirror MongoDB driver semantics
- Full CRUD support: insert, find, update, replace, delete operations
- Environment-driven configuration with no hard-coded defaults
- Comprehensive error handling with proper HTTP status codes
- Structured request/response logging for observability
- Connection pooling and timeout configuration
- Comprehensive test coverage (unit + integration tests)

**Architecture:**
- Built with [Axum](https://github.com/tokio-rs/axum) web framework
- Uses official [MongoDB Rust driver](https://github.com/mongodb/mongo-rust-driver)
- Request/response logging via [tracing](https://github.com/tokio-rs/tracing)
- All configuration via environment variables

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

## API Reference

All JSON requests **must** include `database` and `collection`. Optional `options` maps follow MongoDB driver naming, so fields such as `ordered`, `projection`, `sort`, `upsert`, and `array_filters` behave just like the Rust driver.

**Base URL:** `http://127.0.0.1:3000/api/v1` (or your configured `APP_BIND_ADDRESS`)

**Content-Type:** All requests must include `Content-Type: application/json` header.

### Response Format

Successful responses return JSON with MongoDB driver-shaped payloads:
- Insert operations: `{ "inserted_id": "..." }` or `{ "inserted_ids": [...] }`
- Update/Replace operations: `{ "matched_count": N, "modified_count": N, "upserted_id": "..." }`
- Delete operations: `{ "deleted_count": N }`
- Find operations: `{ "document": {...} }` or `{ "documents": [...] }`

Error responses follow this format:
```json
{
  "error": "error_type",
  "details": "Human-readable error message",
  "correlation_id": "unique-id"
}
```

### Status Codes
- `200 OK` - Successful operation
- `400 Bad Request` - Validation error (missing fields, invalid format)
- `404 Not Found` - Document not found (for single-document operations)
- `502 Bad Gateway` - MongoDB driver/network error
- `500 Internal Server Error` - Unexpected error

## API Quick Reference

### Insert

#### Insert One Document
**Endpoint:** `POST /api/v1/documents/insert-one`

**Request:**
```bash
curl -X POST http://127.0.0.1:3000/api/v1/documents/insert-one \
  -H "Content-Type: application/json" \
  -d '{
    "database": "app",
    "collection": "users",
    "document": {
      "email": "quill@example.com",
      "name": "Quill"
    },
    "options": {
      "ordered": true
    }
  }'
```

**Response (200 OK):**
```json
{
  "inserted_id": "507f1f77bcf86cd799439011"
}
```

#### Insert Many Documents
**Endpoint:** `POST /api/v1/documents/insert-many`

**Request:**
```bash
curl -X POST http://127.0.0.1:3000/api/v1/documents/insert-many \
  -H "Content-Type: application/json" \
  -d '{
    "database": "app",
    "collection": "users",
    "documents": [
      { "email": "rocket@example.com" },
      { "email": "groot@example.com" }
    ],
    "options": {
      "ordered": false
    }
  }'
```

**Response (200 OK):**
```json
{
  "inserted_ids": [
    "507f1f77bcf86cd799439011",
    "507f1f77bcf86cd799439012"
  ]
}
```

### Find

#### Find One Document
**Endpoint:** `POST /api/v1/documents/find-one`

**Request:**
```bash
curl -X POST http://127.0.0.1:3000/api/v1/documents/find-one \
  -H "Content-Type: application/json" \
  -d '{
    "database": "app",
    "collection": "users",
    "filter": { "email": "quill@example.com" },
    "options": {
      "projection": { "_id": 0, "email": 1 }
    }
  }'
```

**Response (200 OK):**
```json
{
  "document": {
    "email": "quill@example.com"
  }
}
```

**Response (404 Not Found):**
```json
{
  "error": "not_found",
  "details": "Document not found",
  "correlation_id": "abc123"
}
```

#### Find Many Documents
**Endpoint:** `POST /api/v1/documents/find-many`

**Request:**
```bash
curl -X POST http://127.0.0.1:3000/api/v1/documents/find-many \
  -H "Content-Type: application/json" \
  -d '{
    "database": "app",
    "collection": "users",
    "filter": { "team": "guardians" },
    "options": {
      "sort": { "created_at": -1 },
      "limit": 50,
      "skip": 0
    }
  }'
```

**Response (200 OK):**
```json
{
  "documents": [
    { "email": "quill@example.com", "team": "guardians" },
    { "email": "rocket@example.com", "team": "guardians" }
  ]
}
```

### Update & Replace

#### Update One Document
**Endpoint:** `POST /api/v1/documents/update-one`

**Request:**
```bash
curl -X POST http://127.0.0.1:3000/api/v1/documents/update-one \
  -H "Content-Type: application/json" \
  -d '{
    "database": "app",
    "collection": "users",
    "filter": { "email": "rocket@example.com" },
    "update": { "$set": { "nickname": "Rocket" } },
    "options": {
      "upsert": false
    }
  }'
```

**Response (200 OK):**
```json
{
  "matched_count": 1,
  "modified_count": 1,
  "upserted_id": null
}
```

#### Update Many Documents
**Endpoint:** `POST /api/v1/documents/update-many`

**Request:**
```bash
curl -X POST http://127.0.0.1:3000/api/v1/documents/update-many \
  -H "Content-Type: application/json" \
  -d '{
    "database": "app",
    "collection": "users",
    "filter": { "team": "guardians" },
    "update": { "$set": { "active": true } }
  }'
```

**Response (200 OK):**
```json
{
  "matched_count": 5,
  "modified_count": 5,
  "upserted_id": null
}
```

#### Replace One Document
**Endpoint:** `POST /api/v1/documents/replace-one`

**Request:**
```bash
curl -X POST http://127.0.0.1:3000/api/v1/documents/replace-one \
  -H "Content-Type: application/json" \
  -d '{
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
  }'
```

**Response (200 OK):**
```json
{
  "matched_count": 1,
  "modified_count": 1,
  "upserted_id": null
}
```

### Delete

#### Delete One Document
**Endpoint:** `POST /api/v1/documents/delete-one`

**Request:**
```bash
curl -X POST http://127.0.0.1:3000/api/v1/documents/delete-one \
  -H "Content-Type: application/json" \
  -d '{
    "database": "app",
    "collection": "users",
    "filter": { "email": "quill@example.com" }
  }'
```

**Response (200 OK):**
```json
{
  "deleted_count": 1
}
```

#### Delete Many Documents
**Endpoint:** `POST /api/v1/documents/delete-many`

**Request:**
```bash
curl -X POST http://127.0.0.1:3000/api/v1/documents/delete-many \
  -H "Content-Type: application/json" \
  -d '{
    "database": "app",
    "collection": "users",
    "filter": { "team": "guardians" }
  }'
```

**Response (200 OK):**
```json
{
  "deleted_count": 5
}
```

### Collections Listing

**Endpoint:** `GET /api/v1/collections?database=app`

**Request:**
```bash
curl http://127.0.0.1:3000/api/v1/collections?database=app
```

**Response (200 OK):**
```json
{
  "collections": ["users", "products", "orders"]
}
```

## Error Handling Examples

### Validation Error (400 Bad Request)
**Request:**
```bash
curl -X POST http://127.0.0.1:3000/api/v1/documents/insert-one \
  -H "Content-Type: application/json" \
  -d '{"collection": "users", "document": {"name": "Alice"}}'
```

**Response (400 Bad Request):**
```json
{
  "error": "validation_error",
  "details": "Missing required field: database",
  "correlation_id": "abc123"
}
```

### MongoDB Connection Error (502 Bad Gateway)
**Response (502 Bad Gateway):**
```json
{
  "error": "mongodb_error",
  "details": "Failed to connect to MongoDB: connection timeout",
  "correlation_id": "def456"
}
```

## Testing

The project includes comprehensive unit and integration tests.

### Running Tests

**All Tests** (unit + integration, recommended):
```bash
cargo test
```

This runs all tests by default. Integration tests automatically skip if MongoDB is not available.

**Unit Tests Only** (no MongoDB required):
```bash
cargo test --lib
```

**Integration Tests Only** (require MongoDB):
```bash
cargo test --tests
# Or more specifically:
cargo test --test integration_test
```

**Note:** Integration tests gracefully skip if MongoDB is not available, so `cargo test` is safe to run without MongoDB. Tests will print "Skipping test: MongoDB not available" for skipped integration tests.

### Test Configuration

Integration tests use the `MONGODB_TEST_URI` environment variable (loaded from `.env` file):
- Default: `mongodb://localhost:27017`
- Can be set in `.env` file: `MONGODB_TEST_URI="mongodb+srv://..."`

### Test Options

**Run specific tests:**
```bash
cargo test test_name_pattern
cargo test --test integration_test test_name_pattern
```

**Show test output:**
```bash
cargo test -- --nocapture
```

**Run tests in parallel (default) or sequentially:**
```bash
cargo test -- --test-threads=1  # Sequential (ensures cleanup runs last)
```

**Run only integration tests:**
```bash
cargo test --tests
```

**Run only unit tests:**
```bash
cargo test --lib
```

### Test Cleanup

After running integration tests, clean up test databases:
```bash
# Run cleanup test (runs last alphabetically)
cargo test --test integration_test --nocapture zzz_cleanup_test_databases

# Or run all tests sequentially with cleanup at the end:
cargo test --test-threads=1 --nocapture
```

Note: The cleanup test is named with `zzz_` prefix to ensure it runs last when tests execute sequentially. Use `--test-threads=1` to ensure sequential execution.

For detailed testing documentation, see [`tests/README.md`](tests/README.md).

## Performance Considerations

- **Connection Pooling:** Configure `MONGODB_POOL_MIN_SIZE` and `MONGODB_POOL_MAX_SIZE` based on your expected load. Default pool sizes are managed by the MongoDB driver.
- **Timeouts:** Set `MONGODB_CONNECT_TIMEOUT_MS` and `MONGODB_SERVER_SELECTION_TIMEOUT_MS` appropriately for your network conditions.
- **Logging:** Adjust `LOG_LEVEL` to `warn` or `error` in production to reduce overhead. Use `info` or `debug` for troubleshooting.
- **Concurrent Requests:** The gateway handles concurrent requests efficiently using Tokio's async runtime. No special configuration needed.

## Security Notes

⚠️ **Important Security Considerations:**

- **No Authentication:** This gateway currently has no authentication or authorization. Do not expose it to untrusted networks without additional security layers (e.g., reverse proxy with auth, VPN, firewall rules).
- **Input Validation:** While the gateway validates required fields, it does not perform deep validation of MongoDB query structures. Ensure your application layer validates user inputs.
- **Connection Strings:** Store MongoDB credentials securely. Never commit `.env` files with credentials to version control.
- **Network Security:** Use TLS/SSL for MongoDB connections (`mongodb+srv://` or `mongodb://...?tls=true`) in production.
- **Rate Limiting:** Consider adding rate limiting at the reverse proxy or application level for production deployments.

## Troubleshooting

### Common Issues

**MongoDB Connection Failures:**
- Verify MongoDB is running and accessible: `mongosh --eval "db.adminCommand('ping')"`
- Check `MONGODB_URI` is correct in `.env` file
- Ensure network connectivity and firewall rules allow connections
- For MongoDB Atlas, verify IP whitelist includes your server's IP

**Startup Failures:**
- Missing required env vars cause immediate failure with clear error messages
- Check logs for specific validation errors
- Ensure `.env` file exists and is properly formatted (no quotes around values unless needed)

**502 Bad Gateway Errors:**
- Usually indicates MongoDB driver/network issues
- Check MongoDB server logs
- Verify connection string format and credentials
- Check correlation ID in error response for tracing

**404 Not Found:**
- Expected for single-document operations when no document matches the filter
- Verify filter criteria matches existing documents
- Check database and collection names are correct

**Test Failures:**
- Integration tests require MongoDB. Set `MONGODB_TEST_URI` if using a non-default instance
- Tests gracefully skip if MongoDB is unavailable
- See `tests/README.md` for detailed testing documentation

### Getting Help

- Check `AGENTS.md` for detailed technical specifications
- Review `docs/request_logging.md` for logging architecture
- If you encounter inconsistencies between this README and `AGENTS.md`, defer to `AGENTS.md` and open a PR to reconcile both
