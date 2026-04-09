#!/bin/bash
set -e

# Make script executable: chmod +x install-service.sh

echo "Installing OMP Discord Bridge as a user-level systemd service..."

# Paths
SERVICE_DIR="${HOME}/.config/systemd/user"
SERVICE_FILE="${SERVICE_DIR}/omp-discord-bridge.service"
PROJECT_DIR=$(pwd)
TARGET_BIN="${PROJECT_DIR}/target/release/omp_discord_bridge"

# Validate
if [ ! -f "$TARGET_BIN" ]; then
    echo "Error: Release binary not found. Please run 'cargo build --release' first."
    exit 1
fi

if [ ! -f "${PROJECT_DIR}/.env" ]; then
    echo "Error: .env file not found in the project directory."
    exit 1
fi

# Ensure user systemd directory exists
mkdir -p "$SERVICE_DIR"

# Get current PATH to ensure bun and omp are available to the service
CURRENT_PATH=$PATH

# Create the service file
cat <<EOF > "$SERVICE_FILE"
[Unit]
Description=Oh My Pi Discord Bridge
After=network.target

[Service]
Type=simple
WorkingDirectory=${PROJECT_DIR}
Environment="PATH=${CURRENT_PATH}"
EnvironmentFile=${PROJECT_DIR}/.env
Environment="RUST_LOG=info"
ExecStart=${TARGET_BIN}
Restart=always
RestartSec=5

[Install]
WantedBy=default.target
EOF

# Reload and enable
systemctl --user daemon-reload
systemctl --user enable omp-discord-bridge.service
systemctl --user restart omp-discord-bridge.service

echo ""
echo "✅ Service installed and started successfully!"
echo ""
echo "You can check the logs anytime with:"
echo "  journalctl --user -u omp-discord-bridge.service -f"
echo ""
echo "To stop the service:"
echo "  systemctl --user stop omp-discord-bridge.service"
