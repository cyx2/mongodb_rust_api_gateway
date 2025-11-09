# Testing Guide

This document covers all aspects of testing in the `hello_rust` project, including unit tests, integration tests, and test configuration options.

## Test Types

### Unit Tests
Unit tests are located inline in source files under `#[cfg(test)]` modules. They test individual functions and modules without requiring external dependencies like MongoDB.

**Run unit tests:**
```bash
cargo test --lib              # Only library unit tests
cargo test module_name::tests # Tests in specific module
```

### Integration Tests
Integration tests are in the `tests/` directory and require MongoDB to be running. They test the full API gateway functionality end-to-end. Integration tests automatically skip gracefully if MongoDB is not available.

**Run integration tests:**
```bash
cargo test --tests                              # All integration tests
cargo test --test integration_test              # Integration test suite
```

### Running All Tests

**Recommended - Run everything:**
```bash
cargo test
```

This runs all unit and integration tests. Integration tests automatically skip if MongoDB is not available, so this command is safe to run without MongoDB.

## Integration Test Setup

The integration tests require MongoDB to be running. By default, they connect to `mongodb://localhost:27017`, but you can specify a different MongoDB instance using the `MONGODB_TEST_URI` environment variable.

## Configuration

Set the `MONGODB_TEST_URI` environment variable to use a different MongoDB instance:

```bash
export MONGODB_TEST_URI="mongodb://localhost:27017"
# Or for a remote instance:
export MONGODB_TEST_URI="mongodb://user:pass@remote-host:27017"
# Or for MongoDB Atlas:
export MONGODB_TEST_URI="mongodb+srv://user:pass@cluster.mongodb.net"
```

If `MONGODB_TEST_URI` is not set, tests default to `mongodb://localhost:27017`.

## Option 1: Using Docker (Recommended)

1. Start Docker Desktop (if not already running)

2. Start MongoDB:
   ```bash
   docker run -d --name mongodb-test -p 27017:27017 mongo:latest
   ```

3. Wait a few seconds for MongoDB to start, then run the tests:
   ```bash
   cargo test --tests
   ```
   
   Or specify a custom MongoDB URI:
   ```bash
   MONGODB_TEST_URI="mongodb://localhost:27017" cargo test --tests
   ```

4. To stop MongoDB when done:
   ```bash
   docker stop mongodb-test
   ```

5. To remove the container:
   ```bash
   docker rm mongodb-test
   ```

## Option 2: Using Homebrew MongoDB

1. Install MongoDB (if not already installed):
   ```bash
   brew tap mongodb/brew
   brew install mongodb-community
   ```

2. Start MongoDB:
   ```bash
   brew services start mongodb-community
   ```

3. Run the tests:
   ```bash
   cargo test --tests
   ```

4. To stop MongoDB:
   ```bash
   brew services stop mongodb-community
   ```

## Option 3: Using System MongoDB

If you have MongoDB installed system-wide:

1. Start MongoDB:
   ```bash
   mongod --dbpath /path/to/data/directory
   ```

2. Run the tests:
   ```bash
   cargo test --tests
   ```

## Verifying MongoDB is Running

You can verify MongoDB is accessible with:
```bash
nc -z localhost 27017 && echo "MongoDB is running" || echo "MongoDB is not accessible"
```

Or using mongosh:
```bash
mongosh --eval "db.adminCommand('ping')"
```

## Running Specific Tests

To run a specific integration test:
```bash
cargo test --test integration_test test_insert_one_and_find_one
```

With a custom MongoDB URI:
```bash
MONGODB_TEST_URI="mongodb://localhost:27017" cargo test --test integration_test test_insert_one_and_find_one
```

## Examples

Run tests against a local MongoDB:
```bash
cargo test --tests
```

Run tests against a remote MongoDB:
```bash
MONGODB_TEST_URI="mongodb://user:pass@remote-host:27017" cargo test --tests
```

Run tests against MongoDB Atlas:
```bash
MONGODB_TEST_URI="mongodb+srv://user:pass@cluster.mongodb.net" cargo test --tests
```

Run tests against a different local port:
```bash
MONGODB_TEST_URI="mongodb://localhost:27018" cargo test --tests
```

## Cleaning Up Test Databases

After running integration tests, test databases (matching the pattern `test_db_*`) are created. To clean them up:

**Run cleanup manually:**
```bash
cargo test --test integration_test --nocapture zzz_cleanup_test_databases
```

The `--nocapture` flag shows the cleanup progress output.

**Run all tests with cleanup automatically at the end:**
```bash
# Sequential execution ensures cleanup runs last
cargo test --test-threads=1 --nocapture
```

**Or run tests then cleanup separately:**
```bash
cargo test --tests
# Then run cleanup:
cargo test --test integration_test --nocapture zzz_cleanup_test_databases
```

The cleanup function will:
- List all databases matching the `test_db_*` pattern
- Drop each test database
- Report success/failure for each database
- Show a summary of cleaned databases

**Note:** 
- Cleanup is safe to run multiple times - it only affects databases matching the test pattern.
- The cleanup test (`zzz_cleanup_test_databases`) is named with a `zzz_` prefix to ensure it runs last alphabetically.
- When using `--test-threads=1` for sequential execution, cleanup will run after all other tests complete.

## Test Command Options

### Cargo Test Options

**Test Selection:**
```bash
cargo test                          # Run all tests (unit + integration, integration skips if MongoDB unavailable)
cargo test --lib                    # Run only unit tests
cargo test --tests                  # Run only integration tests
cargo test test_name_pattern       # Run tests matching pattern
cargo test --test integration_test # Run specific integration test suite
```

**Output Control:**
```bash
cargo test -- --nocapture          # Show stdout/stderr (useful for cleanup output)
cargo test -- --quiet              # Minimal output (one character per test)
cargo test -- --verbose            # More verbose output
```

**Execution Control:**
```bash
cargo test -- --test-threads=1     # Run tests sequentially (useful for debugging)
cargo test -- --no-fail-fast        # Continue running after test failures
cargo test -- --exact               # Match test name exactly
```

**Compilation Only:**
```bash
cargo test --no-run                 # Compile tests but don't run them
```

### Environment Variables

**MONGODB_TEST_URI:**
- Default: `mongodb://localhost:27017`
- Can be set in `.env` file or as environment variable
- Used by integration tests to connect to MongoDB
- Examples:
  - `MONGODB_TEST_URI="mongodb://localhost:27017"`
  - `MONGODB_TEST_URI="mongodb+srv://user:pass@cluster.mongodb.net"`
  - `MONGODB_TEST_URI="mongodb://user:pass@remote-host:27017"`

## Complete Test Workflow

**1. Run all tests (unit + integration, recommended):**
```bash
cargo test
```

Integration tests automatically skip if MongoDB is not available.

**2. Run only unit tests:**
```bash
cargo test --lib
```

**3. Run only integration tests:**
```bash
cargo test --tests
# Or:
cargo test --test integration_test
```

**4. Run specific integration test:**
```bash
cargo test --test integration_test test_insert_one_and_find_one
```

**5. Clean up test databases:**
```bash
# Run cleanup test (runs last alphabetically)
cargo test --test integration_test --nocapture zzz_cleanup_test_databases

# Or run all tests sequentially with cleanup at the end:
cargo test --test-threads=1 --nocapture
```

## CI/CD Considerations

For continuous integration:
```bash
# Format check
cargo fmt --check

# Linting
cargo clippy --tests -- -D warnings

# Unit tests (always run)
cargo test

# Integration tests (if MongoDB available, otherwise they skip gracefully)
cargo test --tests

# Cleanup (optional, for CI environments - runs last when using --test-threads=1)
cargo test --test integration_test --test-threads=1 zzz_cleanup_test_databases
```

## Troubleshooting

**Tests fail with connection errors:**
- Verify MongoDB is running: `nc -z localhost 27017`
- Check `MONGODB_TEST_URI` is set correctly
- Ensure `.env` file is in project root if using it

**Cleanup doesn't find databases:**
- Test databases follow pattern `test_db_*`
- Ensure you're connecting to the same MongoDB instance used for tests
- Check database names with: `mongosh --eval "db.adminCommand('listDatabases')"`

**Tests run slowly:**
- Integration tests require network I/O to MongoDB
- Use `--test-threads=1` to debug, but parallel execution is faster
- Consider using a local MongoDB instance instead of remote

**Test output not showing:**
- Use `--nocapture` flag to see stdout/stderr
- Cleanup output requires `--nocapture` to be visible

