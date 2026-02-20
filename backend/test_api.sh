#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

BASE_URL="http://localhost:3001"
WALLET="AQ68XzKR3fjGypbKi6Ai23vUBTTbEhuKg6EY4uBqAfVY"
TIMESTAMP=$(date +%s)
ASSET_ID="test-asset-$TIMESTAMP"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}   RWA Backend API Test Suite${NC}"
echo -e "${BLUE}========================================${NC}\n"

# Counter for tests
PASSED=0
TOTAL=0

# Function to run test
run_test() {
    local name=$1
    local cmd=$2
    local expected=$3
    
    TOTAL=$((TOTAL+1))
    echo -e "${YELLOW}▶ Test $TOTAL: $name${NC}"
    
    result=$(eval $cmd 2>&1)
    
    if [[ $result == *"$expected"* ]]; then
        echo -e "${GREEN}✅ PASSED${NC}"
        PASSED=$((PASSED+1))
    else
        echo -e "${RED}❌ FAILED${NC}"
        echo "Expected: $expected"
        echo "Got: $result"
    fi
    echo ""
}

# 1. Health Check
run_test "Health Check" \
    "curl -s $BASE_URL/health" \
    "healthy"

# 2. Create Asset
CREATE_CMD="curl -s -X POST $BASE_URL/assets \
    -H \"Content-Type: application/json\" \
    -d '{
        \"asset_id\": \"$ASSET_ID\",
        \"asset_type\": \"real_estate\",
        \"valuation\": 50000000,
        \"metadata_uri\": \"ipfs://QmTest123\",
        \"owner\": \"$WALLET\"
    }'"
run_test "Create Asset" "$CREATE_CMD" "success"

# 3. Get Asset
run_test "Get Asset" \
    "curl -s $BASE_URL/assets/$ASSET_ID" \
    "$ASSET_ID"

# 4. Update Risk Score
UPDATE_CMD="curl -s -X POST $BASE_URL/assets/$ASSET_ID/risk \
    -H \"Content-Type: application/json\" \
    -d '{\"risk_score\": 35}'"
run_test "Update Risk" "$UPDATE_CMD" "success"

# 5. Get Latest Risk
run_test "Get Latest Risk" \
    "curl -s $BASE_URL/assets/$ASSET_ID/risk/latest" \
    "35"

# 6. Create Loan
LOAN_CMD="curl -s -X POST $BASE_URL/loans \
    -H \"Content-Type: application/json\" \
    -d '{
        \"asset_id\": \"$ASSET_ID\",
        \"borrower\": \"$WALLET\",
        \"loan_amount\": 17500000,
        \"interest_rate\": 500,
        \"duration\": 2592000
    }'"
run_test "Create Loan" "$LOAN_CMD" "success"

# 7. Get Risk History
run_test "Get Risk History" \
    "curl -s $BASE_URL/assets/$ASSET_ID/risk/history" \
    "history"

# 8. Chainlink Webhook
WEBHOOK_CMD="curl -s -X POST $BASE_URL/chainlink/webhook \
    -H \"Content-Type: application/json\" \
    -d '{
        \"workflow_id\": \"test-workflow-1\",
        \"asset_id\": \"$ASSET_ID\",
        \"risk_score\": 42,
        \"confidence\": 0.95,
        \"sources\": [\"chainlink\"]
    }'"
run_test "Chainlink Webhook" "$WEBHOOK_CMD" "success"

# Summary
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}✅ Tests Passed: $PASSED/$TOTAL${NC}"
if [ $PASSED -eq $TOTAL ]; then
    echo -e "${GREEN}🎉 ALL TESTS PASSED!${NC}"
else
    echo -e "${RED}❌ Some tests failed${NC}"
fi
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "Test Asset ID: ${YELLOW}$ASSET_ID${NC}"
echo -e "Wallet: ${YELLOW}$WALLET${NC}"
