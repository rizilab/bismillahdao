#!/bin/bash
# Production database initialization script
# This script ensures the database is properly set up

set -e

echo "Checking database initialization..."

# Function to check if database exists and has required extensions
check_database() {
    docker exec bismillahdao-timescaledb psql -U r4gmi -d postgres -t -c "SELECT 1 FROM pg_database WHERE datname='r4gmi'" | grep -q 1
}

# Function to check if TimescaleDB extension is installed
check_timescaledb() {
    docker exec bismillahdao-timescaledb psql -U r4gmi -d r4gmi -t -c "SELECT 1 FROM pg_extension WHERE extname='timescaledb'" | grep -q 1
}

# Main logic
if ! check_database; then
    echo "Database 'r4gmi' does not exist. Creating..."
    docker exec bismillahdao-timescaledb createdb -U r4gmi r4gmi
    echo "Database created successfully."
fi

if ! check_timescaledb; then
    echo "TimescaleDB extension not found. Installing extensions..."
    docker exec bismillahdao-timescaledb psql -U r4gmi -d r4gmi -f /docker-entrypoint-initdb.d/001_create_db.sql
    docker exec bismillahdao-timescaledb psql -U r4gmi -d r4gmi -f /docker-entrypoint-initdb.d/002_extensions.sql
    echo "Extensions installed successfully."
fi

# Run health check
echo "Running health check..."
docker exec bismillahdao-timescaledb psql -U r4gmi -d r4gmi -c "SELECT current_database(), current_user;"

echo "Database initialization completed successfully!" 