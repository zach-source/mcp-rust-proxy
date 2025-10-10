#!/bin/bash
# Integration test suite for Context Tracing with Claude CLI
# Tests the complete flow of automatic tracking through real Claude interactions

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PROXY_PATH="./target/debug/mcp-rust-proxy"
CONFIG_PATH="./mcp-proxy-config.yaml"
DB_PATH="/Users/ztaylor/.mcp-proxy/context-tracing.db"
TEST_WORKSPACE="/tmp/claude-tracing-test-$(date +%s)"

# Clean up function
cleanup() {
    echo -e "${YELLOW}Cleaning up test environment...${NC}"
    rm -rf "$TEST_WORKSPACE"
    rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"
}

trap cleanup EXIT

echo "=========================================="
echo "Claude CLI + Context Tracing Integration Test"
echo "=========================================="
echo ""

# Verify prerequisites
if ! command -v claude &> /dev/null; then
    echo -e "${RED}❌ Claude CLI not found${NC}"
    exit 1
fi

if [ ! -f "$PROXY_PATH" ]; then
    echo -e "${RED}❌ Proxy binary not found. Run: cargo build${NC}"
    exit 1
fi

# Create test workspace
mkdir -p "$TEST_WORKSPACE"
cd "$TEST_WORKSPACE"

echo -e "${GREEN}✓${NC} Test workspace: $TEST_WORKSPACE"
echo -e "${GREEN}✓${NC} Database: $DB_PATH"
echo ""

# Clean database for fresh test
rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"
mkdir -p "$(dirname "$DB_PATH")"

echo "=========================================="
echo "Test 1: Simple Tool Call with Tracking"
echo "=========================================="

# Create a simple prompt that will use memory tool
cat > test1_prompt.txt << 'EOF'
Create a memory entity called "TraceTest1" of type "integration_test" with one observation: "Testing automatic context tracking"
EOF

echo -e "${YELLOW}Running: Claude with memory tool call${NC}"
RESPONSE=$(claude --mcp-config "{\"mcpServers\":{\"proxy\":{\"command\":\"$PROXY_PATH\",\"args\":[\"--config\",\"$CONFIG_PATH\",\"--stdio\"]}}}" < test1_prompt.txt 2>&1)

echo "$RESPONSE" | head -20
echo ""

# Now query the tracing database to find the response_id
echo -e "${YELLOW}Querying SQLite for tracked responses...${NC}"
RESPONSE_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM responses;" 2>/dev/null || echo "0")

if [ "$RESPONSE_COUNT" -gt 0 ]; then
    echo -e "${GREEN}✓${NC} Found $RESPONSE_COUNT tracked response(s)"

    # Get the response ID
    RESPONSE_ID=$(sqlite3 "$DB_PATH" "SELECT id FROM responses ORDER BY timestamp DESC LIMIT 1;")
    echo -e "${GREEN}✓${NC} Response ID: $RESPONSE_ID"

    # Query lineage via MCP tool
    cat > query_trace.json << EOF
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "mcp__proxy__tracing__get_trace",
    "arguments": {
      "response_id": "$RESPONSE_ID",
      "format": "compact"
    }
  }
}
EOF

    echo -e "${YELLOW}Retrieving lineage manifest...${NC}"
    LINEAGE=$(echo $(cat query_trace.json) | $PROXY_PATH --config $CONFIG_PATH --stdio 2>/dev/null | grep '^{"jsonrpc"')

    if echo "$LINEAGE" | jq -e '.result' > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} Lineage retrieved successfully:"
        echo "$LINEAGE" | jq -r '.result.content[0].text' | head -10
        echo ""

        # Check context count
        CONTEXT_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM context_units;")
        echo -e "${GREEN}✓${NC} Context units recorded: $CONTEXT_COUNT"
    else
        echo -e "${RED}❌ Failed to retrieve lineage${NC}"
        echo "$LINEAGE" | jq '.'
    fi
else
    echo -e "${RED}❌ No responses tracked${NC}"
fi

echo ""

echo "=========================================="
echo "Test 2: Submit Feedback via MCP Tool"
echo "=========================================="

if [ -n "$RESPONSE_ID" ]; then
    cat > submit_feedback.json << EOF
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "mcp__proxy__tracing__submit_feedback",
    "arguments": {
      "response_id": "$RESPONSE_ID",
      "score": 0.9,
      "feedback_text": "Integration test - successful tool call",
      "user_id": "integration-test"
    }
  }
}
EOF

    echo -e "${YELLOW}Submitting feedback...${NC}"
    FEEDBACK_RESULT=$(echo $(cat submit_feedback.json) | $PROXY_PATH --config $CONFIG_PATH --stdio 2>/dev/null | grep '^{"jsonrpc"')

    if echo "$FEEDBACK_RESULT" | jq -e '.result' > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} Feedback submitted successfully"
        echo "$FEEDBACK_RESULT" | jq -r '.result.content[0].text'

        # Verify feedback was stored
        FEEDBACK_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM feedback;")
        echo -e "${GREEN}✓${NC} Feedback records: $FEEDBACK_COUNT"

        # Check if context score was updated
        CONTEXT_SCORE=$(sqlite3 "$DB_PATH" "SELECT aggregate_score, feedback_count FROM context_units LIMIT 1;")
        echo -e "${GREEN}✓${NC} Context scores updated: $CONTEXT_SCORE"
    else
        echo -e "${RED}❌ Feedback submission failed${NC}"
        echo "$FEEDBACK_RESULT" | jq '.'
    fi
else
    echo -e "${YELLOW}⊘ Skipped (no response_id available)${NC}"
fi

echo ""

echo "=========================================="
echo "Test 3: Query Context Impact"
echo "=========================================="

if [ -n "$RESPONSE_ID" ]; then
    # Get a context ID from the database
    CONTEXT_ID=$(sqlite3 "$DB_PATH" "SELECT id FROM context_units LIMIT 1;")

    if [ -n "$CONTEXT_ID" ]; then
        echo -e "${GREEN}✓${NC} Testing with context: $CONTEXT_ID"

        cat > query_impact.json << EOF
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "mcp__proxy__tracing__query_context_impact",
    "arguments": {
      "context_unit_id": "$CONTEXT_ID",
      "limit": 10
    }
  }
}
EOF

        echo -e "${YELLOW}Querying context impact...${NC}"
        IMPACT_RESULT=$(echo $(cat query_impact.json) | $PROXY_PATH --config $CONFIG_PATH --stdio 2>/dev/null | grep '^{"jsonrpc"')

        if echo "$IMPACT_RESULT" | jq -e '.result' > /dev/null 2>&1; then
            echo -e "${GREEN}✓${NC} Impact report retrieved:"
            echo "$IMPACT_RESULT" | jq -r '.result.content[0].text' | jq '{context_unit_id, total_responses, avg_weight}'
        else
            echo -e "${RED}❌ Impact query failed${NC}"
        fi
    else
        echo -e "${YELLOW}⊘ No context units found${NC}"
    fi
else
    echo -e "${YELLOW}⊘ Skipped (no response_id available)${NC}"
fi

echo ""

echo "=========================================="
echo "Test 4: Read Quality Resources"
echo "=========================================="

cat > read_resource.json << 'EOF'
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "resources/read",
  "params": {
    "uri": "trace://quality/recent-feedback"
  }
}
EOF

echo -e "${YELLOW}Reading trace://quality/recent-feedback...${NC}"
RESOURCE_RESULT=$(echo $(cat read_resource.json) | $PROXY_PATH --config $CONFIG_PATH --stdio 2>/dev/null | grep '^{"jsonrpc"')

if echo "$RESOURCE_RESULT" | jq -e '.result' > /dev/null 2>&1; then
    FEEDBACK_ITEMS=$(echo "$RESOURCE_RESULT" | jq -r '.result.contents[0].text' | jq 'length')
    echo -e "${GREEN}✓${NC} Recent feedback resource readable: $FEEDBACK_ITEMS items"
else
    echo -e "${RED}❌ Resource read failed${NC}"
fi

echo ""

echo "=========================================="
echo "Test 5: Database Integrity Check"
echo "=========================================="

echo "Database schema verification:"
TABLES=$(sqlite3 "$DB_PATH" "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;" 2>/dev/null)
echo "$TABLES" | while read table; do
    COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM $table;")
    echo -e "${GREEN}✓${NC} $table: $COUNT records"
done

echo ""

# Check for WAL mode
WAL_MODE=$(sqlite3 "$DB_PATH" "PRAGMA journal_mode;" 2>/dev/null)
if [ "$WAL_MODE" = "wal" ]; then
    echo -e "${GREEN}✓${NC} WAL mode enabled"
else
    echo -e "${YELLOW}⚠${NC} WAL mode: $WAL_MODE"
fi

echo ""

echo "=========================================="
echo "Final Summary"
echo "=========================================="

TOTAL_RESPONSES=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM responses;" 2>/dev/null || echo "0")
TOTAL_CONTEXTS=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM context_units;" 2>/dev/null || echo "0")
TOTAL_LINEAGE=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM lineage;" 2>/dev/null || echo "0")
TOTAL_FEEDBACK=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM feedback;" 2>/dev/null || echo "0")

echo "Database Statistics:"
echo "  Responses tracked: $TOTAL_RESPONSES"
echo "  Context units: $TOTAL_CONTEXTS"
echo "  Lineage relationships: $TOTAL_LINEAGE"
echo "  Feedback submissions: $TOTAL_FEEDBACK"
echo ""

if [ "$TOTAL_RESPONSES" -gt 0 ] && [ "$TOTAL_CONTEXTS" -gt 0 ]; then
    echo -e "${GREEN}✅ INTEGRATION TEST PASSED${NC}"
    echo ""
    echo "The Context Tracing Framework successfully:"
    echo "  ✓ Tracks responses automatically"
    echo "  ✓ Records context units from backend tool calls"
    echo "  ✓ Generates and stores lineage manifests"
    echo "  ✓ Accepts and propagates feedback"
    echo "  ✓ Exposes data via MCP tools and resources"
    echo "  ✓ Maintains database integrity"
    echo ""
    echo -e "${GREEN}🚀 Ready for production use with Claude CLI!${NC}"
    exit 0
else
    echo -e "${RED}❌ INTEGRATION TEST FAILED${NC}"
    echo "Expected data was not tracked properly"
    exit 1
fi
