#!/bin/bash
# Automated integration test for Context Tracing Framework
# Simulates Claude CLI interactions without requiring actual Claude

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROXY="./target/debug/mcp-rust-proxy"
CONFIG="mcp-proxy-config.yaml"
DB="/Users/ztaylor/.mcp-proxy/context-tracing.db"

echo -e "${BLUE}=========================================="
echo "Automated Context Tracing Test Suite"
echo -e "==========================================${NC}"
echo ""

# Clean database
echo -e "${YELLOW}Preparing test environment...${NC}"
rm -f "$DB" "$DB-shm" "$DB-wal"
mkdir -p "$(dirname "$DB")"

# Helper function
run_mcp_request() {
    local request="$1"
    echo "$request" | $PROXY --config $CONFIG --stdio 2>&1 | grep '^{"jsonrpc"'
}

PASSED=0
FAILED=0

# Test function
run_test() {
    local test_name="$1"
    local test_func="$2"

    echo -e "${BLUE}Test: $test_name${NC}"
    if $test_func; then
        echo -e "${GREEN}✓ PASSED${NC}"
        ((PASSED++))
    else
        echo -e "${RED}✗ FAILED${NC}"
        ((FAILED++))
    fi
    echo ""
}

# Test 1: Verify tracing tools are listed
test_tools_list() {
    local response=$(run_mcp_request '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}')
    local tracing_count=$(echo "$response" | jq -r '.result.tools | map(select(.name | startswith("mcp__proxy__tracing"))) | length')

    if [ "$tracing_count" -eq 5 ]; then
        echo "  Found all 5 tracing tools"
        return 0
    else
        echo "  Expected 5 tracing tools, found $tracing_count"
        return 1
    fi
}

# Test 2: Verify tracing resources are listed
test_resources_list() {
    local response=$(run_mcp_request '{"jsonrpc":"2.0","id":2,"method":"resources/list","params":{}}')
    local resource_count=$(echo "$response" | jq -r '.result.resources | length')

    if [ "$resource_count" -eq 4 ]; then
        echo "  Found all 4 tracing resources"
        return 0
    else
        echo "  Expected 4 tracing resources, found $resource_count"
        return 1
    fi
}

# Test 3: Call backend tool and verify tracking
test_automatic_tracking() {
    # Call memory tool
    local response=$(run_mcp_request '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"mcp__proxy__memory__create_entities","arguments":{"entities":[{"name":"AutoTrackedTest","entityType":"test","observations":["Automatically tracked"]}]}}}')

    if ! echo "$response" | jq -e '.result' > /dev/null 2>&1; then
        echo "  Tool call failed"
        return 1
    fi

    # Wait a moment for async storage
    sleep 1

    # Check database
    local response_count=$(sqlite3 "$DB" "SELECT COUNT(*) FROM responses;" 2>/dev/null || echo "0")
    local context_count=$(sqlite3 "$DB" "SELECT COUNT(*) FROM context_units;" 2>/dev/null || echo "0")

    if [ "$response_count" -gt 0 ] && [ "$context_count" -gt 0 ]; then
        echo "  Responses tracked: $response_count"
        echo "  Contexts recorded: $context_count"
        return 0
    else
        echo "  No automatic tracking occurred"
        return 1
    fi
}

# Test 4: Retrieve lineage via MCP tool
test_lineage_retrieval() {
    local response_id=$(sqlite3 "$DB" "SELECT id FROM responses ORDER BY timestamp DESC LIMIT 1;" 2>/dev/null)

    if [ -z "$response_id" ]; then
        echo "  No response to query"
        return 1
    fi

    local response=$(run_mcp_request "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"mcp__proxy__tracing__get_trace\",\"arguments\":{\"response_id\":\"$response_id\",\"format\":\"json\"}}}")

    if echo "$response" | jq -e '.result.content[0].text' > /dev/null 2>&1; then
        local context_count=$(echo "$response" | jq -r '.result.content[0].text | fromjson | .context_tree | length')
        echo "  Retrieved lineage with $context_count contexts"
        return 0
    else
        echo "  Failed to retrieve lineage"
        return 1
    fi
}

# Test 5: Submit feedback and verify propagation
test_feedback_propagation() {
    local response_id=$(sqlite3 "$DB" "SELECT id FROM responses ORDER BY timestamp DESC LIMIT 1;" 2>/dev/null)

    if [ -z "$response_id" ]; then
        echo "  No response for feedback"
        return 1
    fi

    # Get initial context score
    local initial_score=$(sqlite3 "$DB" "SELECT aggregate_score FROM context_units LIMIT 1;" 2>/dev/null)

    # Submit feedback
    local response=$(run_mcp_request "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"tools/call\",\"params\":{\"name\":\"mcp__proxy__tracing__submit_feedback\",\"arguments\":{\"response_id\":\"$response_id\",\"score\":0.85,\"feedback_text\":\"Test feedback\"}}}")

    if ! echo "$response" | jq -e '.result' > /dev/null 2>&1; then
        echo "  Feedback submission failed"
        return 1
    fi

    # Verify score changed
    local new_score=$(sqlite3 "$DB" "SELECT aggregate_score FROM context_units LIMIT 1;" 2>/dev/null)

    if [ "$new_score" != "$initial_score" ]; then
        echo "  Score propagated: $initial_score → $new_score"
        return 0
    else
        echo "  Score did not change"
        return 1
    fi
}

# Test 6: Read quality resource
test_quality_resource() {
    local response=$(run_mcp_request '{"jsonrpc":"2.0","id":6,"method":"resources/read","params":{"uri":"trace://quality/recent-feedback"}}')

    if echo "$response" | jq -e '.result.contents[0].text' > /dev/null 2>&1; then
        local feedback_array=$(echo "$response" | jq -r '.result.contents[0].text')
        echo "  Resource readable, feedback data returned"
        return 0
    else
        echo "  Resource read failed"
        return 1
    fi
}

# Test 7: Database integrity
test_database_integrity() {
    # Check all tables exist
    local tables=$(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;" 2>/dev/null)
    local expected_tables=("context_units" "feedback" "lineage" "lineage_manifests" "responses")

    for table in "${expected_tables[@]}"; do
        if ! echo "$tables" | grep -q "^$table$"; then
            echo "  Missing table: $table"
            return 1
        fi
    done

    # Check WAL mode
    local wal_mode=$(sqlite3 "$DB" "PRAGMA journal_mode;" 2>/dev/null)
    if [ "$wal_mode" != "wal" ]; then
        echo "  WAL mode not enabled"
        return 1
    fi

    echo "  All tables present, WAL mode enabled"
    return 0
}

# Run all tests
echo -e "${BLUE}=========================================="
echo "Running Test Suite"
echo -e "==========================================${NC}"
echo ""

run_test "1. Tracing tools listed" test_tools_list
run_test "2. Tracing resources listed" test_resources_list
run_test "3. Automatic tracking on tool call" test_automatic_tracking
run_test "4. Lineage retrieval via MCP" test_lineage_retrieval
run_test "5. Feedback propagation" test_feedback_propagation
run_test "6. Quality resource readable" test_quality_resource
run_test "7. Database integrity" test_database_integrity

echo -e "${BLUE}=========================================="
echo "Test Results"
echo -e "==========================================${NC}"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ ALL TESTS PASSED${NC}"
    echo ""
    echo "Context Tracing Framework is fully operational:"
    echo "  ✓ Automatic tracking working"
    echo "  ✓ Lineage manifests generated"
    echo "  ✓ Feedback loop functional"
    echo "  ✓ MCP integration complete"
    echo "  ✓ Database integrity maintained"
    echo ""
    echo -e "${GREEN}🚀 Production ready!${NC}"
    exit 0
else
    echo -e "${RED}❌ SOME TESTS FAILED${NC}"
    echo "Review errors above and fix issues"
    exit 1
fi
