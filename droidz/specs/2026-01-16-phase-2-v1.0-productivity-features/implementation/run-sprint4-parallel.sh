#!/bin/bash
# Sprint 4 Parallel Implementation Execution
# Runs Clipboard History and Calculator implementations concurrently

set -e

SPEC_DIR="droidz/specs/2026-01-16-phase-2-v1.0-productivity-features"
PROMPTS_DIR="$SPEC_DIR/implementation/prompts"
CONFIG_FILE="droidz/config.yml"

echo "🚀 Starting Sprint 4 parallel implementation..."
echo "📋 Processing 2 task groups concurrently:"
echo "   • 4.1 Clipboard History (42 tasks)"
echo "   • 4.2 Calculator (47 tasks)"
echo ""

# Load Factory API key from config.yml or environment
if [ -f "$CONFIG_FILE" ]; then
  echo "📄 Loading configuration from $CONFIG_FILE"
  
  # Extract API key from YAML (simple grep approach)
  API_KEY=$(grep "^factory_api_key:" "$CONFIG_FILE" 2>/dev/null | sed 's/factory_api_key:[[:space:]]*//' | tr -d '"' | tr -d "'" || true)
  
  # Extract optional settings
  AUTONOMY=$(grep "^default_autonomy_level:" "$CONFIG_FILE" 2>/dev/null | sed 's/default_autonomy_level:[[:space:]]*//' | tr -d '"' | tr -d "'" || echo "full")
  
  # Use config file API key if set, otherwise fall back to env var
  if [ -n "$API_KEY" ]; then
    export FACTORY_API_KEY="$API_KEY"
    echo "✅ Using API key from config.yml"
  fi
else
  # Use defaults if no config file
  AUTONOMY="full"
fi

# Check that we have an API key from either source
if [ -z "$FACTORY_API_KEY" ]; then
  echo "❌ Error: No Factory API key found"
  echo ""
  echo "Option 1 (Recommended): Add to config file"
  echo "   1. Create droidz/config.yml with:"
  echo "      factory_api_key: \"fk-your-key-here\""
  echo "   2. Get your key from: https://app.factory.ai/settings/api-keys"
  echo ""
  echo "Option 2: Use environment variable"
  echo "   export FACTORY_API_KEY=fk-..."
  echo ""
  exit 1
fi

echo "⚙️  Autonomy level: $AUTONOMY"
echo ""

# Create log directory
LOG_DIR="$SPEC_DIR/implementation/logs"
mkdir -p "$LOG_DIR"

# Run both implementations in parallel
echo "▶️  Starting parallel execution..."
echo ""

# Start Clipboard History implementation
echo "[Clipboard History] Starting..."
droid exec --auto "$AUTONOMY" -f "$PROMPTS_DIR/1-clipboard-history.md" > "$LOG_DIR/clipboard-history.log" 2>&1 &
PID1=$!

# Start Calculator implementation
echo "[Calculator] Starting..."
droid exec --auto "$AUTONOMY" -f "$PROMPTS_DIR/2-calculator.md" > "$LOG_DIR/calculator.log" 2>&1 &
PID2=$!

echo ""
echo "📊 Running implementations:"
echo "   PID $PID1: Clipboard History → $LOG_DIR/clipboard-history.log"
echo "   PID $PID2: Calculator → $LOG_DIR/calculator.log"
echo ""
echo "⏳ Waiting for completion... (this may take a while)"
echo "   Tip: tail -f $LOG_DIR/*.log to watch progress"
echo ""

# Wait for both to complete
FAIL=0

wait $PID1 || {
  echo "❌ Clipboard History failed (exit code: $?)"
  FAIL=1
}

wait $PID2 || {
  echo "❌ Calculator failed (exit code: $?)"
  FAIL=1
}

echo ""
if [ $FAIL -eq 0 ]; then
  echo "🎉 Sprint 4 implementation completed successfully!"
  echo ""
  echo "📝 Check results:"
  echo "   • tasks.md - Updated with [x] for completed tasks"
  echo "   • crates/photoncast-clipboard/ - Clipboard History implementation"
  echo "   • crates/photoncast-calculator/ - Calculator implementation"
  echo ""
  echo "🔍 Review logs:"
  echo "   • $LOG_DIR/clipboard-history.log"
  echo "   • $LOG_DIR/calculator.log"
else
  echo "⚠️  Some implementations failed. Check logs for details:"
  echo "   • $LOG_DIR/clipboard-history.log"
  echo "   • $LOG_DIR/calculator.log"
  exit 1
fi
