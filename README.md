# MongoDB API Gateway

This project provides an Axum-based API gateway that aggregates CRUD access to every collection in a MongoDB database. The service inspects the database on startup using the `MONGODB_URI` connection string and exposes RESTful endpoints for interacting with the discovered collections.

## Prerequisites

- Rust toolchain (see [rustup.rs](https://rustup.rs))
- Access to a MongoDB instance
- `MONGODB_URI` environment variable or `.env` file containing the connection string with a default database specified.

## Getting Started

1. (Optional) Create a `.env` file with the following content:
   ```env
   MONGODB_URI=mongodb+srv://<user>:<password>@<cluster>/<database>?retryWrites=true&w=majority
   PORT=3000
   ```

2. Install dependencies and run the service:
   ```bash
   cargo run
   ```

   The server listens on `0.0.0.0:3000` by default. Override the port with the `PORT` environment variable if needed.

## API Overview

The gateway automatically discovers collections and offers CRUD endpoints for each collection:

- `GET /health` &mdash; Service health probe.
- `GET /collections` &mdash; List the available collection names.
- `GET /collections/{collection}?limit=&skip=` &mdash; Fetch documents from a collection with optional pagination.
- `POST /collections/{collection}` &mdash; Insert a new document (provide a JSON payload).
- `GET /collections/{collection}/{id}` &mdash; Retrieve a document by identifier (`ObjectId` strings are parsed automatically, all other identifiers are treated as strings).
- `PUT /collections/{collection}/{id}` &mdash; Apply a partial update to a document using `$set` semantics.
- `DELETE /collections/{collection}/{id}` &mdash; Remove a document.

All responses are serialized as JSON. Insert and update operations return counts or identifiers from MongoDB. CORS is fully permissive, making the gateway easy to integrate with web clients.

## Graceful Shutdown

The application listens for `CTRL+C` and (on Unix platforms) `SIGTERM` to terminate gracefully, allowing in-flight requests to complete.

## Logging

Logging is managed through `tracing`. Control verbosity using the `RUST_LOG` environment variable, for example:

```bash
RUST_LOG=info cargo run
```
