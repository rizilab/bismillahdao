# Muhafidh Project

Muhafidh is the guardian layer of BismillahDAO, providing security analysis for Solana tokens.

## Components

- **Raqib**: Transaction Monitor
- **Baseer**: Wallet Analyzer
- **Siraaj**: Token Analyzer

## Database Migrations

The project uses a dedicated migration system to manage database schema changes:

### Migration Architecture

We follow the "shared migration code but separate execution" approach:

1. Migrations are defined in `src/storage/migration.rs`
2. A standalone CLI tool (`migrate`) applies migrations
3. Services check schema version but don't attempt migrations

### Running Migrations

Before starting any service, run the migration tool:

```bash
cargo run --bin migrate
```

This ensures the database is properly initialized and updated to the latest schema.

### Deployment Workflow

In production environments:

1. Run `migrate` as an initialization step
2. Start services (raqib, baseer, siraaj) only after migrations complete
3. Services will verify the schema version matches what they expect

If using Docker, create a dedicated migration container that runs before the service containers.

## Configuration

Configuration is loaded from `Config.toml` by default.

## Running Services

Start each service separately:

```bash
cargo run --bin raqib
cargo run --bin baseer
cargo run --bin siraaj
```

Make sure to run the migration tool first:

```bash
cargo run --bin migrate
```

## Testing

Muhafidh uses a comprehensive Test-Driven Development (TDD) approach with multiple testing strategies to ensure reliability in the complex async, concurrent, and distributed blockchain analysis environment.

### Prerequisites

Before running tests, ensure you have the required dependencies:

```bash
# Install testing tools
cargo install just
cargo install cargo-llvm-cov
cargo install cargo-watch

# Start test infrastructure (Docker required)
just setup-test-env
```

### Quick Start Testing

For immediate testing, run these commands:

```bash
# 1. Run all tests with coverage
just test-all

# 2. Run tests in watch mode during development
just test-watch

# 3. Generate coverage report
just coverage-html
```

### Step-by-Step Testing Guide

#### 1. Environment Setup

```bash
# Clean any existing test artifacts
just clean-test

# Set up test environment with Docker containers
just setup-test-env

# Verify test database is running
just test-db-status
```

#### 2. Basic Testing Workflow

```bash
# Step 1: Run unit tests (fastest feedback)
just test-unit

# Step 2: Run integration tests (with real database)
just test-integration

# Step 3: Run all tests together
just test-all
```

#### 3. Specialized Testing

```bash
# Property-based testing (automatic test case generation)
just test-property

# BFS race condition testing (critical for blockchain analysis)
just test-bfs

# Concurrency testing (async/await validation)
just test-concurrency

# Stress testing (high-load scenarios)
just test-stress
```

#### 4. Performance Testing

```bash
# Run benchmarks
just bench

# Profile memory usage
just test-memory

# Performance monitoring
just bench-creator
just bench-bfs
```

#### 5. Coverage and Reporting

```bash
# Generate HTML coverage report
just coverage-html

# Generate coverage summary
just coverage

# View detailed test results
just test-report
```

### Test Categories Explained

#### Unit Tests (`tests/unit_*.rs`)
- **Purpose**: Test individual components in isolation
- **Features**: Uses mocks for external dependencies
- **Run**: `just test-unit`
- **Example**: Testing creator handler logic without database

#### Integration Tests (`tests/integration_*.rs`)
- **Purpose**: Test component interactions with real services
- **Features**: Uses Docker containers for databases
- **Run**: `just test-integration`
- **Example**: Testing creator metadata storage with PostgreSQL

#### Property Tests (`tests/property_*.rs`)
- **Purpose**: Automatic test case generation with random inputs
- **Features**: Uses `proptest` for property verification
- **Run**: `just test-property`
- **Example**: Testing BFS operations with various graph structures

#### Stress Tests (`tests/stress_*.rs`)
- **Purpose**: High-concurrency and race condition testing
- **Features**: Simulates production-like load
- **Run**: `just test-stress`
- **Example**: Testing BFS race conditions with circular transfers

#### Benchmarks (`tests/benchmark_*.rs`)
- **Purpose**: Performance measurement and regression detection
- **Features**: Uses `criterion` for statistical analysis
- **Run**: `just bench`
- **Example**: Measuring creator processing throughput

### Development Workflow

#### Continuous Testing During Development

```bash
# Watch for changes and run tests automatically
just test-watch

# Watch specific test files
just test-watch-unit
just test-watch-integration
```

#### Pre-Commit Testing

```bash
# Quick validation before committing
just test-quick

# Full validation (recommended)
just test-ci
```

### Understanding Test Results

#### Coverage Reports
- **Location**: `test-results/coverage/`
- **View**: Open `test-results/coverage/index.html` in browser
- **Target**: Aim for >80% coverage on critical paths

#### Benchmark Results
- **Location**: `test-results/benchmarks/`
- **Format**: HTML reports with statistical analysis
- **Monitoring**: Look for performance regressions

#### Test Logs
- **Location**: `test-results/logs/`
- **Purpose**: Debugging failed tests
- **Levels**: DEBUG, INFO, WARN, ERROR

### Troubleshooting Common Issues

#### Database Connection Issues
```bash
# Check database status
just test-db-status

# Restart test database
just test-db-restart

# View database logs
docker logs muhafidh-test-postgres
```

#### Memory Issues in Tests
```bash
# Run with memory profiling
just test-memory

# Clean up test artifacts
just clean-test

# Restart test environment
just setup-test-env
```

#### Race Condition Debugging
```bash
# Run with detailed logging
RUST_LOG=debug just test-bfs

# Run multiple times to catch intermittent issues
just test-stress-repeat

# Enable trace-level logging for BFS
RUST_LOG=muhafidh::bfs=trace just test-bfs
```

### CI/CD Integration

The justfile includes commands for continuous integration:

```bash
# CI-friendly test run (no interactive output)
just test-ci

# Generate reports for CI artifacts
just coverage-ci
just bench-ci
```

### Advanced Testing Features

#### Custom Test Selection
```bash
# Run specific test files
cargo test integration_creator_metadata

# Run tests matching pattern
cargo test creator

# Run ignored tests (long-running)
cargo test -- --ignored
```

#### Test Configuration
- **Test Fixtures**: Located in `tests/fixtures/`
- **Mock Data**: Configurable via environment variables
- **Timeouts**: Adjustable per test category

### Testing Best Practices

1. **Start with Unit Tests**: Fastest feedback loop
2. **Use Integration Tests**: For component interactions
3. **Property Tests for Edge Cases**: Automatic boundary testing
4. **Stress Tests for Production**: Simulate real-world load
5. **Monitor Coverage**: Maintain >80% on critical paths
6. **Benchmark Regularly**: Catch performance regressions early

For more detailed information about the testing infrastructure, see the individual test files in the `tests/` directory.
