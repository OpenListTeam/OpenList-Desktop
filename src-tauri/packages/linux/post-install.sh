#!/bin/bash

# Migration: Clean up old installation in /usr/bin if it exists
# This handles upgrades from previous versions that installed to /usr/bin
if [ -f "/usr/bin/openlist-desktop" ] && [ ! -L "/usr/bin/openlist-desktop" ]; then
    echo "Migrating from old installation location (/usr/bin) to /opt/OpenList-Desktop"
    # Remove old binaries that are now in /opt
    rm -f /usr/bin/install-openlist-service
    rm -f /usr/bin/uninstall-openlist-service
    rm -f /usr/bin/openlist-desktop-service
    rm -f /usr/bin/openlist
    rm -f /usr/bin/rclone
    # Note: /usr/bin/openlist-desktop will be replaced with symlink below
fi

# Set execute permissions for binaries in /opt
chmod +x /opt/OpenList-Desktop/install-openlist-service
chmod +x /opt/OpenList-Desktop/uninstall-openlist-service
chmod +x /opt/OpenList-Desktop/openlist-desktop-service

# Create symlink for easy command-line access
# Using -f flag to force creation even if file exists
ln -sf /opt/OpenList-Desktop/openlist-desktop /usr/bin/openlist-desktop
