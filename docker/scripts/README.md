# Bismillahdao Service Stop Cron Job

This directory contains scripts to automatically stop the `bismillahdao-raqib` and `bismillahdao-baseer` services daily at 11:00 PM UTC+7 (Asia/Jakarta timezone).

## Files

- `stop-services-cron.sh` - The main cron job script that stops the services
- `setup-cron.sh` - Setup script to configure the cron job
- `README.md` - This documentation file

## Quick Setup

### For Production Environment

```bash
# Navigate to the scripts directory
cd /path/to/docker/scripts

# Run the setup script as root
sudo ./setup-cron.sh production
```

### For Development Environment

```bash
# Navigate to the scripts directory
cd /path/to/docker/scripts

# Run the setup script as root
sudo ./setup-cron.sh development
```

## Manual Setup

If you prefer to set up the cron job manually:

1. Make the script executable:
   ```bash
   chmod +x stop-services-cron.sh
   ```

2. Add the cron job entry:
   ```bash
   sudo crontab -e
   ```

3. Add one of these lines depending on your server timezone:

   **If your server is in UTC+7 timezone:**
   ```
   0 23 * * * /path/to/docker/scripts/stop-services-cron.sh production >> /var/log/bismillahdao-cron.log 2>&1
   ```

   **If your server is in UTC timezone:**
   ```
   0 16 * * * /path/to/docker/scripts/stop-services-cron.sh production >> /var/log/bismillahdao-cron.log 2>&1
   ```

## Script Features

### stop-services-cron.sh

- **Dual Strategy**: First tries to stop services using docker-compose, then falls back to direct docker commands
- **Environment Support**: Supports both production and development environments
- **Comprehensive Logging**: All operations are logged with timestamps
- **Error Handling**: Graceful error handling and informative messages
- **Permission Checks**: Warns if not running with proper permissions

### setup-cron.sh

- **Timezone Detection**: Automatically detects server timezone and configures appropriate cron timing
- **Prerequisites Check**: Verifies Docker installation and script availability
- **Permission Management**: Sets up proper file permissions
- **Cron Management**: Handles existing cron jobs and prevents duplicates
- **Testing**: Tests the script during setup to ensure it works

## Usage Examples

### Test the script manually

```bash
# Test in production mode
./stop-services-cron.sh production

# Test in development mode
./stop-services-cron.sh development

# Show help
./stop-services-cron.sh --help
```

### Check cron job status

```bash
# View current cron jobs
sudo crontab -l

# Check if the job is scheduled
sudo crontab -l | grep stop-services-cron
```

### Monitor logs

```bash
# View recent log entries
sudo tail -f /var/log/bismillahdao-cron.log

# View all logs
sudo cat /var/log/bismillahdao-cron.log
```

## Timezone Information

The cron job is configured to run at **11:00 PM UTC+7** (Asia/Jakarta timezone).

- **UTC+7 servers**: Cron runs at `23:00` (11:00 PM local time)
- **UTC servers**: Cron runs at `16:00` (4:00 PM UTC = 11:00 PM UTC+7)

## Troubleshooting

### Script doesn't run

1. Check if the cron service is running:
   ```bash
   sudo systemctl status cron
   ```

2. Verify the cron job is installed:
   ```bash
   sudo crontab -l
   ```

3. Check script permissions:
   ```bash
   ls -la stop-services-cron.sh
   ```

### Services don't stop

1. Check if the containers exist:
   ```bash
   docker ps -a | grep -E "(bismillahdao-raqib|bismillahdao-baseer)"
   ```

2. Verify docker-compose file exists:
   ```bash
   ls -la ../docker-compose.*.yaml
   ```

3. Test manual stop:
   ```bash
   docker stop bismillahdao-raqib bismillahdao-baseer
   ```

### Log issues

1. Check log file permissions:
   ```bash
   ls -la /var/log/bismillahdao-cron.log
   ```

2. Ensure log directory exists:
   ```bash
   sudo mkdir -p /var/log
   ```

## Removing the Cron Job

To remove the cron job:

```bash
# Edit crontab
sudo crontab -e

# Delete the line containing "stop-services-cron.sh"
# Save and exit
```

Or use this one-liner:

```bash
(sudo crontab -l | grep -v "stop-services-cron.sh") | sudo crontab -
```

## Security Considerations

- The scripts are designed to run as root for proper Docker access
- Log files are created with restricted permissions (644)
- Scripts validate inputs and handle errors gracefully
- No sensitive information is logged

## Environment Differences

### Production
- Uses `docker-compose.prod.yaml` if available
- Falls back to direct docker commands
- More conservative error handling

### Development
- Uses `docker-compose.dev.yaml`
- Includes additional development-specific services
- More verbose logging for debugging

## Support

For issues or questions:
1. Check the logs first: `/var/log/bismillahdao-cron.log`
2. Test the script manually to isolate issues
3. Verify Docker and docker-compose installations
4. Check network connectivity and service definitions 