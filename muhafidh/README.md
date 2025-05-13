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
