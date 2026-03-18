#!/bin/bash
set -euo pipefail

# Test the webhook forwarder locally.
# Start the server first with: cargo run
# Then run this script in another terminal.

BASE="http://localhost:8080"

echo "=== Test 1: GET should return 405 ==="
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/test-token")
echo "GET /test-token -> $STATUS (expect 405)"

echo ""
echo "=== Test 2: POST to /<token> should forward ==="
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/test-token" -H "Content-Type: application/json" -d '{"ref":"refs/heads/main"}')
echo "POST /test-token -> $STATUS (expect 502 if Dokploy unreachable, or upstream status)"

echo ""
echo "=== Test 3: POST to /compose/<token> should forward ==="
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/compose/test-token" -H "Content-Type: application/json" -d '{"ref":"refs/heads/main"}')
echo "POST /compose/test-token -> $STATUS (expect 502 if Dokploy unreachable, or upstream status)"

echo ""
echo "=== Test 4: POST to invalid path should return 404 ==="
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/")
echo "POST / -> $STATUS (expect 404)"

echo ""
echo "=== Test 5: POST to nested invalid path should return 404 ==="
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/foo/bar/baz")
echo "POST /foo/bar/baz -> $STATUS (expect 404)"

echo ""
echo "=== Test 6: POST form-urlencoded payload should be converted to JSON ==="
PAYLOAD=$(python3 -c "import urllib.parse; print(urllib.parse.urlencode({'payload': '{\"ref\":\"refs/heads/main\"}'}))") 
RESP=$(curl -s -w "\n%{http_code}" -X POST "$BASE/test-token" -H "Content-Type: application/x-www-form-urlencoded" -H "X-GitHub-Event: push" -d "$PAYLOAD")
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -n -1)
echo "POST /test-token (form-urlencoded) -> $STATUS (expect 502 if Dokploy unreachable, or upstream status)"
echo "Body: $BODY"

echo ""
echo "Done!"
