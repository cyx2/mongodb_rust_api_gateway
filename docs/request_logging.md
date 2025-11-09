# Request Logging Architecture

This document captures the structure and rationale behind the per-request logging that wraps every MongoDB gateway endpoint. It serves as both a reference for understanding the current implementation and a guide for extending logging in the future.

## Objectives

- Surface actionable telemetry for each HTTP call so operators can trace payloads, status codes, and failure reasons without enabling verbose driver logs.
- Keep instrumentation orthogonal to business logic by centralizing reusable helpers rather than duplicating logging statements across handlers.
- Preserve the existing error model and response contract while enriching emitted spans and log records with endpoint context.

## Entry Points

All instrumentation is owned by `src/routes.rs` where every handler resides.

- `router` wires all REST paths through the shared `AppState` and is the sole place routes are registered. This guarantees that helpers declared in the same module are available to every endpoint.
- Each handler is annotated with `#[instrument(skip_all)]` from `tracing` to suppress automatic argument recording. This keeps the span lean while allowing manual control over which payload fields are logged.

## Logging Helpers

Three helper functions shape the lifecycle logs for a request:

1. `log_request_received(endpoint, request)` – emits an `info` log on the `http` target as soon as Axum deserializes the payload. The handler passes the strongly typed request body, which derives `Debug`, so JSON fields appear in structured form. [`src/routes.rs`](../src/routes.rs)
2. `log_request_success(endpoint, status, response)` – records successful completions, including the final HTTP status and the serialized response payload. Responses are wrapped in `axum::Json`, so the helper accepts anything implementing `Debug` without leaking internal state. [`src/routes.rs`](../src/routes.rs)
3. `log_request_failure(endpoint, error)` – promotes failure handling to a single branch. It logs a `warn` record tagged with the failing endpoint, the error’s HTTP status (exposed via `ApiError::status`), and the full `ApiError` for diagnostics before returning the same error to Axum. [`src/routes.rs`](../src/routes.rs)

By returning the error from `log_request_failure`, handlers can use the helper inline inside `Result::map_err` without altering the control flow. This keeps MongoDB driver errors and validation failures flowing through the existing `ApiResult` alias while attaching consistent telemetry.

## Handler Flow

Every handler follows the same structure:

1. Call `log_request_received` immediately after destructuring the `Json<T>` payload.
2. Use `collection_from_state` to resolve the MongoDB collection, logging validation failures if the namespace is incomplete.
3. Execute the driver call, mapping driver errors through `log_request_failure` after wrapping them with `map_driver_error` to sanitize the message.
4. Construct the response DTO and pass it to `log_request_success` with the appropriate `StatusCode` before returning it wrapped in `Json`.

This pattern produces a minimal log sequence for the happy path and ensures every early return or `Err` branch emits a failure log.

## Error Surface

`ApiError::status` moved out of the `#[cfg(test)]` block in `src/error.rs` so the logging helpers can emit the HTTP code associated with each error variant. The actual JSON response remains unchanged because `IntoResponse` is still implemented directly on `ApiError`. [`src/error.rs`](../src/error.rs)

## Configuration Integration

`src/main.rs` continues to drive runtime configuration through `Config::from_env`. Log verbosity is controlled via the existing `LOG_LEVEL` environment variable, interpreted by `tracing_subscriber::EnvFilter`. Operators can increase verbosity to `debug` or `trace` without code changes. [`src/main.rs`](../src/main.rs)

## Extending the Pattern

- New endpoints should live in `src/routes.rs`, call the same helper trio, and prefer returning `ApiResult<Json<_>>` so logging stays uniform.
- Cross-cutting metadata (e.g., correlation IDs) can be added by extending the helpers, giving all handlers richer logs with a single change.
- When adding request types, ensure `Debug` is derived so payloads render cleanly in logs. The `models` module already derives `Debug` for every request/response struct. [`src/models.rs`](../src/models.rs)

## Operational Impact

The logging scheme provides a balanced trade-off between observability and noise:

- Operators see exactly three structured log lines per request (receive, success, or failure) on the `info` channel, making it easy to follow individual calls in production logs.
- Validation issues (400), not-found cases (404), and driver failures (502) now emit `warn` logs that include the HTTP code and full error object, enabling faster debugging without inspecting HTTP traces.
- Because the helpers avoid capturing references to the `AppState` or MongoDB client, there is no risk of accidental data cloning or contention in the logging path.

This documentation should be reviewed alongside the route module when introducing new endpoints to ensure future work continues to conform to the established logging contract.

## Log Format Examples

### Successful Request Log
```
[INFO] http: request_received endpoint=/api/v1/documents/insert-one request=InsertOneRequest { database: "app", collection: "users", document: {...}, options: None }
[INFO] http: request_success endpoint=/api/v1/documents/insert-one status=200 response=InsertOneResponse { inserted_id: "507f1f77bcf86cd799439011" }
```

### Failed Request Log
```
[INFO] http: request_received endpoint=/api/v1/documents/find-one request=FindOneRequest { database: "app", collection: "users", filter: {...}, options: None }
[WARN] http: request_failure endpoint=/api/v1/documents/find-one status=404 error=ApiError::NotFound { details: "Document not found", correlation_id: "abc123" }
```

## Log Levels

- **`info`**: Normal request/response flow (receive, success)
- **`warn`**: Error conditions (validation failures, not found, MongoDB errors)
- **`error`**: Reserved for unexpected panics or critical system failures (not currently used)
- **`debug`**: Detailed tracing information (can be enabled via `LOG_LEVEL=debug`)
- **`trace`**: Very verbose tracing (can be enabled via `LOG_LEVEL=trace`)

## Configuration

Logging is configured via the `LOG_LEVEL` environment variable:
- Default: `info` (shows request/receive/success/failure logs)
- `debug`: Includes additional tracing information
- `trace`: Maximum verbosity
- `warn`: Only warnings and errors
- `error`: Only errors

Set in `.env` file:
```bash
LOG_LEVEL=info
```

## Best Practices

### When Adding New Endpoints
1. Always use the three logging helpers (`log_request_received`, `log_request_success`, `log_request_failure`)
2. Call `log_request_received` immediately after extracting the request body
3. Call `log_request_success` before returning successful responses
4. Use `log_request_failure` in all error paths via `map_err`
5. Ensure request/response types derive `Debug` for clean log output

### Logging Sensitive Data
- **Never log:** Passwords, API keys, tokens, full connection strings
- **Sanitize:** MongoDB connection strings should be logged with credentials masked
- **Filter:** Consider adding log filtering middleware for production to redact sensitive fields

### Performance Considerations
- Structured logging has minimal overhead
- Use appropriate log levels in production (`warn` or `error`)
- Avoid logging extremely large payloads (consider truncation for large documents)
- Correlation IDs enable efficient log aggregation and tracing

## Integration with Log Aggregation

The structured logging format is designed to work with log aggregation systems:

### JSON Format (Future Enhancement)
Consider configuring `tracing_subscriber` to output JSON format:
```rust
tracing_subscriber::fmt()
    .json()
    .init();
```

This enables:
- Easy parsing by log aggregation tools (ELK, Splunk, Datadog, etc.)
- Structured querying and filtering
- Correlation ID-based trace reconstruction

### Correlation IDs
Each error response includes a `correlation_id` that can be used to:
- Trace a request through multiple services
- Correlate logs with error responses
- Debug production issues efficiently

## Troubleshooting

### Logs Not Appearing
- Check `LOG_LEVEL` environment variable is set correctly
- Verify `tracing_subscriber` is initialized in `main.rs`
- Ensure logs aren't being filtered by log aggregation system

### Too Much Log Noise
- Reduce `LOG_LEVEL` to `warn` or `error` in production
- Consider filtering specific endpoints if needed
- Review log output format and adjust verbosity

### Missing Context
- Ensure all handlers use the logging helpers
- Check that request/response types derive `Debug`
- Verify endpoint names are consistent across logs

## Future Enhancements

Potential improvements to consider:
- [ ] JSON log format for better aggregation
- [ ] Request ID middleware for automatic correlation ID generation
- [ ] Log sampling for high-traffic endpoints
- [ ] Structured fields for better querying (user_id, database, collection, etc.)
- [ ] Log filtering middleware for sensitive data redaction
- [ ] Metrics integration (request counts, latencies, error rates)
- [ ] Distributed tracing support (OpenTelemetry, Jaeger, etc.)
