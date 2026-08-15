#!/bin/bash
echo "Starting test run at: $(date '+%Y-%m-%d %H:%M:%S')"
npm run test:gui:all
echo "Finished test run at: $(date '+%Y-%m-%d %H:%M:%S')"
