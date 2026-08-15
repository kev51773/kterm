#!/bin/bash
echo "Started build at: $(date '+%Y-%m-%d %H:%M:%S')"
npx tauri build --no-bundle
echo "Finished build at: $(date '+%Y-%m-%d %H:%M:%S')"
