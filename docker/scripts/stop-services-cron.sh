#!/bin/bash

# Cron job script to stop bismillahdao-raqib and bismillahdao-baseer services
# This script should be run daily at 11PM UTC+7

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$(dirname "$SCRIPT_DIR")"
LOG_FILE="/var/log/bismillahdao-cron.log"

# Determine environment and compose file
ENVIRONMENT="${1:-production}"
if [ "$ENVIRONMENT" = "development" ]; then
    COMPOSE_FILE="$DOCKER_DIR/docker-compose.dev.yaml"
else
    COMPOSE_FILE="$DOCKER_DIR/docker-compose.prod.yaml"
fi

# Services to stop
SERVICES=("bismillahdao-raqib" "bismillahdao-baseer")

# Function to log messages with timestamp
log_message() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') [$$] - $1" | tee -a "$LOG_FILE"
}

# Function to stop a service using direct docker command
stop_service() {
    local service_name="$1"
    log_message "Attempting to stop container: $service_name"
    
    # Check if container exists and is running
    if docker ps --format "table {{.Names}}" | grep -q "^$service_name$"; then
        log_message "Container $service_name is running. Stopping..."
        
        # Stop the container
        if docker stop "$service_name"; then
            log_message "Successfully stopped container: $service_name"
            return 0
        else
            log_message "ERROR: Failed to stop container: $service_name"
            return 1
        fi
    else
        log_message "Container $service_name is not running or does not exist"
        return 0
    fi
}

# Function to stop services using docker-compose
stop_services_compose() {
    log_message "Attempting to stop services using docker-compose"
    log_message "Using compose file: $COMPOSE_FILE"
    
    if [ -f "$COMPOSE_FILE" ]; then
        cd "$DOCKER_DIR" || {
            log_message "ERROR: Cannot change to docker directory: $DOCKER_DIR"
            return 1
        }
        
        # Try to stop services that are defined in compose
        compose_success=0  # 0 = success in bash
        for service in "${SERVICES[@]}"; do
            log_message "Stopping service $service via docker-compose"
            if docker-compose -f "$(basename "$COMPOSE_FILE")" stop "$service" 2>/dev/null; then
                log_message "Successfully stopped service via compose: $service"
            else
                log_message "WARNING: Service $service not found in compose file or failed to stop"
                compose_success=1  # 1 = failure in bash
            fi
        done
        
        return $compose_success
    else
        log_message "WARNING: Docker compose file not found: $COMPOSE_FILE"
        return 1
    fi
}

# Function to check if running as root (recommended for cron jobs)
check_permissions() {
    if [ "$EUID" -ne 0 ]; then
        log_message "WARNING: Not running as root. Docker commands may fail if user doesn't have proper permissions"
    fi
}

# Main execution
main() {
    log_message "Starting daily service stop routine (Environment: $ENVIRONMENT)"
    log_message "Target services: ${SERVICES[*]}"
    
    # Check permissions
    check_permissions
    
    # Create log directory if it doesn't exist
    LOG_DIR="$(dirname "$LOG_FILE")"
    if [ ! -d "$LOG_DIR" ]; then
        mkdir -p "$LOG_DIR"
    fi
    
    # Create log file if it doesn't exist
    if [ ! -f "$LOG_FILE" ]; then
        touch "$LOG_FILE"
        chmod 644 "$LOG_FILE"
    fi
    
    # Try stopping services using docker-compose first
    if stop_services_compose; then
        log_message "Docker-compose stop completed successfully"
    else
        log_message "Docker-compose stop had issues, trying direct container stop as fallback"
        
        # Try stopping containers directly as backup
        for service in "${SERVICES[@]}"; do
            stop_service "$service"
        done
    fi
    
    log_message "Daily service stop routine completed"
    log_message "----------------------------------------"
}

# Show usage if help is requested
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "Usage: $0 [environment]"
    echo "  environment: 'production' (default) or 'development'"
    echo "  Stops bismillahdao-raqib and bismillahdao-baseer services"
    echo "  Logs to: $LOG_FILE"
    exit 0
fi

# Run main function
main "$@" 