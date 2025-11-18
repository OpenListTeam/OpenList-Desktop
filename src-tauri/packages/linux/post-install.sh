#!/bin/bash
chmod +x /opt/OpenList-Desktop/install-openlist-service
chmod +x /opt/OpenList-Desktop/uninstall-openlist-service
chmod +x /opt/OpenList-Desktop/openlist-desktop-service

# Create symlink for easy command-line access
ln -sf /opt/OpenList-Desktop/openlist-desktop /usr/bin/openlist-desktop
