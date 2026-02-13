#!/bin/bash
# Phase implementation automation loop
# Automatically works through specs/command-line-interface.md phase by phase

set -e  # Exit on error (except where explicitly handled)

# Configuration
SPEC_FILE="$(pwd)/specs/command-line-interface.md"
DELAY=3
MAX_RETRIES=3

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print functions
print_header() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Verify spec file exists
if [ ! -f "$SPEC_FILE" ]; then
    print_error "Spec file not found: $SPEC_FILE"
    exit 1
fi

# Main loop
print_header "🤖 Phase Automation Started"
echo "📋 Spec: $SPEC_FILE"
echo "⏱️  Delay: ${DELAY}s between iterations"
echo "🔄 Max retries: $MAX_RETRIES"
echo ""

iteration=0
retry_count=0

while true; do
    iteration=$((iteration + 1))

    echo ""
    echo -e "${BLUE}━━━━━━━━━━ Iteration #$iteration [$(date +%H:%M:%S)] ━━━━━━━━━━${NC}"
    echo "🚀 Starting next deliverable..."
    echo ""

    # Run Claude with the workflow prompt
    OUTPUT=$(claude --dangerously-skip-permissions -p "Read ${SPEC_FILE}.

WORKFLOW (complete ONE deliverable per run):
1. Find first phase with unchecked [ ] deliverables
2. Check git branch - if not on phase branch, create 'phase-N-name' and checkout
3. Find first [ ] deliverable in that phase
4. Implement it FULLY - write ACTUAL CODE in src/ or crates/
5. Write tests in tests/ or #[cfg(test)] modules
6. Run 'cargo test --package coding-agent-cli' until passing
7. Run 'cargo build --package coding-agent-cli' - verify no warnings
8. Mark item as [x] in ${SPEC_FILE}
9. Git add all changes, commit: '<action>: <what> (Phase N.M)'
10. If ALL phase items [x]: merge to main, push, delete branch
11. Push current branch to origin
12. Output 'DELIVERABLE_COMPLETE' and exit

CRITICAL RULES:
- Write REAL CODE, not just mark checkboxes
- Do NOT mark [x] until code written AND tests pass
- Fix any compilation warnings before marking complete
- Work on ONE deliverable only, then EXIT
- If no unchecked [ ] items remain anywhere, output 'ALL_PHASES_COMPLETE' and exit

Exit immediately after completing one deliverable." 2>&1)

    EXIT_CODE=$?

    # Check for completion markers
    if echo "$OUTPUT" | grep -q "ALL_PHASES_COMPLETE"; then
        echo ""
        print_header "🎉 ALL PHASES COMPLETE!"
        print_success "All deliverables implemented and tested"
        print_success "Automation finished at $(date)"
        exit 0
    fi

    # Check for successful deliverable completion
    if [ $EXIT_CODE -eq 0 ] && echo "$OUTPUT" | grep -q "DELIVERABLE_COMPLETE"; then
        print_success "Deliverable completed successfully"
        retry_count=0  # Reset retry counter on success
    else
        print_warning "Exit code: $EXIT_CODE"
        retry_count=$((retry_count + 1))

        if [ $retry_count -ge $MAX_RETRIES ]; then
            print_error "Failed $MAX_RETRIES consecutive times"
            echo ""
            echo "Last output (last 30 lines):"
            echo "─────────────────────────────────────────"
            echo "$OUTPUT" | tail -30
            echo "─────────────────────────────────────────"
            echo ""
            print_error "Manual intervention required. Exiting."
            exit 1
        fi

        print_warning "Retry $retry_count/$MAX_RETRIES in ${DELAY}s..."
    fi

    # Show current git status
    echo ""
    echo "📊 Current state:"
    echo "   Branch: $(git branch --show-current)"
    echo "   Iteration: #$iteration"
    echo "   Retry count: $retry_count/$MAX_RETRIES"

    echo ""
    echo -e "${YELLOW}⏳ Waiting ${DELAY}s before next iteration...${NC}"
    sleep $DELAY
done

print_header "🏁 Automation loop finished - $(date)"
