#!/bin/bash

set -a
source .env
set +a

# --- API Throughput Benchmark Script ---
# This script performs load testing using the OHA utility.

API_URL="${BENCH_URL}"
USERNAME="${BENCH_USER}"
PASSWORD="${BENCH_PASS}"

echo "Getting JWT token for $USERNAME ..."

# Login request
TOKEN=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -d "{\"email\": \"$USERNAME\", \"password\": \"$PASSWORD\"}" \
  "$API_URL/login" | jq -r '.result.token')

# Check token
if [ -z "$TOKEN" ] || [ "$TOKEN" == "null" ]; then
    echo "Failed to get JWT token"
    exit 1
fi
echo "Token successfully received."
echo "--------------------------------------"
echo "Running load test on endpoint '/users/me' (simple request)..."

# Test a simple protected endpoint
oha -c 100 -z 10s -H "Authorization: Bearer $TOKEN" "$API_URL/users/me"

echo ""
echo "--------------------------------------"
echo "Running load test on endpoint '/servers' (DB query speed)..."

# Test an endpoint with heavy database interaction
oha -c 100 -z 10s -H "Authorization: Bearer $TOKEN" "$API_URL/servers"
