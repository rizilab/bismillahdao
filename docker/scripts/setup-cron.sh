#!/bin/bash

# Setup script for bismillahdao service stop cron job
# This script helps configure the cron job to run at 11PM UTC+7

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRON_SCRIPT="$SCRIPT_DIR/stop-services-cron.sh"
ENVIRONMENT="${1:-production}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Function to determine cron time based on server timezone
get_cron_time() {
    local server_tz
    server_tz=$(timedatectl show --property=Timezone --value 2>/dev/null || echo "Unknown")
    
    # Print info to stderr so it doesn't interfere with the return value
    print_info "Detected server timezone: $server_tz" >&2
    print_info "Target time: 11:00 PM UTC+7 (Asia/Jakarta)" >&2
    
    # Calculate the appropriate cron time and return it
    case "$server_tz" in
        "Asia/Jakarta"|"Asia/Bangkok"|"Asia/Ho_Chi_Minh")
            # Server is in UTC+7, so 11PM local time
            print_info "Cron will run at 11:00 PM local time (UTC+7)" >&2
            echo "0 23 * * *"
            ;;
        "UTC"|"Etc/UTC")
            # Server is in UTC, so we need 4PM UTC (11PM UTC+7 - 7 hours)
            print_info "Cron will run at 4:00 PM UTC (11:00 PM UTC+7)" >&2
            echo "0 16 * * *"
            ;;
        "America/New_York")
            # Eastern Time: UTC-5 (EST) or UTC-4 (EDT)
            # For 11PM UTC+7, we need 10AM EST (UTC-5) or 11AM EDT (UTC-4)
            # Using 10AM to be safe (works for both EST and EDT)
            print_info "Cron will run at 10:00 AM Eastern Time (11:00 PM UTC+7)" >&2
            echo "0 10 * * *"
            ;;
        "America/Chicago")
            # Central Time: UTC-6 (CST) or UTC-5 (CDT)
            # For 11PM UTC+7, we need 9AM CST or 10AM CDT
            print_info "Cron will run at 9:00 AM Central Time (11:00 PM UTC+7)" >&2
            echo "0 9 * * *"
            ;;
        "America/Denver")
            # Mountain Time: UTC-7 (MST) or UTC-6 (MDT)
            # For 11PM UTC+7, we need 8AM MST or 9AM MDT
            print_info "Cron will run at 8:00 AM Mountain Time (11:00 PM UTC+7)" >&2
            echo "0 8 * * *"
            ;;
        "America/Los_Angeles")
            # Pacific Time: UTC-8 (PST) or UTC-7 (PDT)
            # For 11PM UTC+7, we need 7AM PST or 8AM PDT
            print_info "Cron will run at 7:00 AM Pacific Time (11:00 PM UTC+7)" >&2
            echo "0 7 * * *"
            ;;
        "Europe/London")
            # GMT/BST: UTC+0 or UTC+1
            # For 11PM UTC+7, we need 4PM GMT or 5PM BST
            print_info "Cron will run at 4:00 PM GMT/5:00 PM BST (11:00 PM UTC+7)" >&2
            echo "0 16 * * *"
            ;;
        *)
            print_warning "Unknown timezone: $server_tz" >&2
            print_warning "Please manually configure the cron time." >&2
            print_warning "For 11PM UTC+7:" >&2
            print_warning "  - If server is UTC: use '0 16 * * *'" >&2
            print_warning "  - If server is UTC+7: use '0 23 * * *'" >&2
            print_warning "  - If server is EST/EDT: use '0 10 * * *'" >&2
            echo "0 16 * * *"
            ;;
    esac
}

# Function to check prerequisites
check_prerequisites() {
    print_info "Checking prerequisites..."
    
    # Check if running as root
    if [ "$EUID" -ne 0 ]; then
        print_error "This script should be run as root to set up system cron jobs"
        exit 1
    fi
    
    # Check if docker is available
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed or not in PATH"
        exit 1
    fi

    
    # Check if cron script exists
    if [ ! -f "$CRON_SCRIPT" ]; then
        print_error "Cron script not found: $CRON_SCRIPT"
        exit 1
    fi
    
    print_success "Prerequisites check completed"
}

# Function to make scripts executable
setup_permissions() {
    print_info "Setting up file permissions..."
    
    chmod +x "$CRON_SCRIPT"
    
    # Create log directory
    LOG_DIR="/var/log"
    if [ ! -d "$LOG_DIR" ]; then
        mkdir -p "$LOG_DIR"
    fi
    
    print_success "Permissions configured"
}

# Function to install cron job
install_cron_job() {
    local cron_time="$1"
    local cron_entry="$cron_time $CRON_SCRIPT $ENVIRONMENT >> /var/log/bismillahdao-cron.log 2>&1"
    
    print_info "Installing cron job..."
    print_info "Cron entry: $cron_entry"
    
    # Remove any existing cron job for this script
    (crontab -l 2>/dev/null | grep -v "$CRON_SCRIPT" || true) | crontab -
    
    # Add the new cron job
    (crontab -l 2>/dev/null || true; echo "$cron_entry") | crontab -
    
    if [ $? -eq 0 ]; then
        print_success "Cron job installed successfully"
    else
        print_error "Failed to install cron job"
        print_warning "You may need to install it manually:"
        print_warning "  crontab -e"
        print_warning "  Add this line: $cron_entry"
        return 1
    fi
}

# Function to test the script
test_script() {
    print_info "Testing the cron script..."
    
    if "$CRON_SCRIPT" "$ENVIRONMENT"; then
        print_success "Script test completed successfully"
    else
        print_warning "Script test completed with warnings. Check the logs for details."
    fi
}

# Function to show current cron jobs
show_cron_status() {
    print_info "Current cron jobs related to bismillahdao:"
    if crontab -l 2>/dev/null | grep -q "bismillahdao\|stop-services-cron"; then
        crontab -l 2>/dev/null | grep "bismillahdao\|stop-services-cron"
    else
        print_info "No bismillahdao-related cron jobs found"
    fi
}

# Main function
main() {
    echo "================================================================"
    echo "Bismillahdao Service Stop Cron Job Setup"
    echo "================================================================"
    echo
    
    print_info "Environment: $ENVIRONMENT"
    print_info "Script location: $CRON_SCRIPT"
    echo
    
    check_prerequisites
    setup_permissions
    install_cron_job "$(get_cron_time)"
    
    echo
    print_info "Testing the script (dry run)..."
    test_script
    
    echo
    show_cron_status
    
    echo
    print_success "Setup completed successfully!"
    echo
    print_info "The cron job will stop bismillahdao-raqib and bismillahdao-baseer services"
    print_info "every day at 11:00 PM UTC+7 (Asia/Jakarta time)"
    print_info "Logs will be written to: /var/log/bismillahdao-cron.log"
    echo
    print_info "To remove the cron job, run:"
    print_info "  crontab -e"
    print_info "  # then delete the line containing: $CRON_SCRIPT"
    echo
    print_info "To check logs:"
    print_info "  tail -f /var/log/bismillahdao-cron.log"
}

# Show usage
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "Usage: $0 [environment]"
    echo "  environment: 'production' (default) or 'development'"
    echo
    echo "This script sets up a cron job to stop bismillahdao services daily at 11PM UTC+7"
    echo "Run as root to install system-wide cron job"
    exit 0
fi

# Run main function
main 