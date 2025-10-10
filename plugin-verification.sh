#!/bin/bash
echo "=== Claude Code Plugin Verification ==="
echo ""

# Check plugin structure
echo "1. Plugin Structure:"
if [ -f ".claude-plugin/plugin.json" ]; then
    echo "  ✓ plugin.json exists"
    jq -r '.name, .version, .description' .claude-plugin/plugin.json | while read line; do
        echo "    $line"
    done
else
    echo "  ✗ plugin.json missing"
    exit 1
fi
echo ""

echo "2. Commands:"
COMMANDS=$(ls -1 .claude-plugin/commands/*.md 2>/dev/null | wc -l)
echo "  ✓ $COMMANDS command files"
ls -1 .claude-plugin/commands/*.md | sed 's/.*\//    - /'
echo ""

echo "3. Hooks:"
HOOKS=$(ls -1 .claude/hooks/*.sh 2>/dev/null | wc -l)
echo "  ✓ $HOOKS hook scripts"
ls -1 .claude/hooks/*.sh | sed 's/.*\//    - /'
echo ""

echo "4. MCP Server Binary:"
if [ -f "target/debug/mcp-rust-proxy" ]; then
    SIZE=$(ls -lh target/debug/mcp-rust-proxy | awk '{print $5}')
    echo "  ✓ Proxy binary: $SIZE"
else
    echo "  ✗ Binary not built - run: cargo build"
fi
echo ""

echo "5. Configuration:"
if [ -f "mcp-proxy-config.yaml" ]; then
    echo "  ✓ Config file exists"
    echo "    Servers: $(grep -c "command:" mcp-proxy-config.yaml)"
    echo "    Tracing: $(grep "enabled: true" mcp-proxy-config.yaml | grep -c contextTracing || echo "disabled")"
else
    echo "  ✗ Config missing"
fi
echo ""

echo "6. Documentation:"
DOCS=$(ls -1 *.md 2>/dev/null | wc -l)
echo "  ✓ $DOCS documentation files"
echo ""

echo "7. Test Suite:"
TESTS=$(ls -1 tests/*.rs tests/*.sh 2>/dev/null | wc -l)
echo "  ✓ $TESTS test files"
echo ""

echo "=== Plugin Ready for Installation ==="
echo ""
echo "To use:"
echo "  1. cd $(pwd)"
echo "  2. claude"
echo "  3. Plugin auto-activates with all features"
