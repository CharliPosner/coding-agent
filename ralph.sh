#!/bin/bash
# ralph.sh - General-purpose Ralph loop for any spec file
#
# Usage:
#   ./ralph.sh <spec-file>                 # Loop until spec complete (in worktree)
#   ./ralph.sh <spec-file> --once          # Single iteration only
#   ./ralph.sh <spec-file> --no-worktree   # Loop in current directory (no isolation)
#
# Examples:
#   ./ralph.sh specs/auth.md               # Loop on auth until complete
#   ./ralph.sh specs/ui.md &               # Background loop on UI spec
#   ./ralph.sh specs/api.md &              # Parallel loop on API spec (different worktree)
#   ./ralph.sh specs/auth.md --once        # Debug: run one iteration only
#
# Each spec runs in its own worktree, so multiple loops can run simultaneously.

set -e

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Configuration
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

WORKTREE_DIR=".worktrees"
DELAY=3
MAX_RETRIES=3

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Colors
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Helpers
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

print_header() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_success() { echo -e "${GREEN}✓ $1${NC}"; }
print_warning() { echo -e "${YELLOW}⚠ $1${NC}"; }
print_error()   { echo -e "${RED}✗ $1${NC}"; }
print_info()    { echo -e "${CYAN}→ $1${NC}"; }

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Parse arguments
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

SPEC_FILE=""
LOOP_MODE=true
USE_WORKTREE=true

while [[ $# -gt 0 ]]; do
    case $1 in
        --once)
            LOOP_MODE=false
            shift
            ;;
        --no-worktree)
            USE_WORKTREE=false
            shift
            ;;
        --help|-h)
            echo "Usage: $0 <spec-file> [--once] [--no-worktree]"
            echo ""
            echo "Arguments:"
            echo "  spec-file       Path to the spec/implementation plan"
            echo ""
            echo "Options:"
            echo "  --once          Run single iteration only (for debugging)"
            echo "  --no-worktree   Run in current directory (no isolation)"
            echo ""
            echo "Examples:"
            echo "  $0 specs/auth.md              # Loop until complete"
            echo "  $0 specs/auth.md &            # Background loop"
            echo "  $0 specs/auth.md --once       # Single iteration (debug)"
            exit 0
            ;;
        *)
            if [[ -z "$SPEC_FILE" ]]; then
                SPEC_FILE="$1"
            else
                echo "Unknown argument: $1"
                exit 1
            fi
            shift
            ;;
    esac
done

if [[ -z "$SPEC_FILE" ]]; then
    print_error "No spec file provided"
    echo "Usage: $0 <spec-file> [--loop] [--no-worktree]"
    exit 1
fi

if [[ ! -f "$SPEC_FILE" ]]; then
    print_error "Spec file not found: $SPEC_FILE"
    exit 1
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Derive names from spec file
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

# Get absolute paths
REPO_ROOT="$(git rev-parse --show-toplevel)"
SPEC_FILE_ABS="$(cd "$(dirname "$SPEC_FILE")" && pwd)/$(basename "$SPEC_FILE")"

# Derive branch/worktree name from spec filename
# specs/command-line-interface.md → cli
# specs/auth.md → auth
# specs/image-generation.md → image-generation
SPEC_NAME="$(basename "$SPEC_FILE" .md)"
SPEC_NAME="${SPEC_NAME/command-line-interface/cli}"  # Shorten common names

BRANCH_NAME="ralph-${SPEC_NAME}"
WORKTREE_PATH="${REPO_ROOT}/${WORKTREE_DIR}/${SPEC_NAME}"

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Worktree management
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

setup_worktree() {
    if [[ "$USE_WORKTREE" != true ]]; then
        echo "$REPO_ROOT"
        return
    fi

    mkdir -p "$REPO_ROOT/$WORKTREE_DIR"

    if [[ -d "$WORKTREE_PATH" ]]; then
        print_info "Using existing worktree: $WORKTREE_PATH" >&2
        echo "$WORKTREE_PATH"
        return
    fi

    print_info "Creating worktree for $SPEC_NAME..." >&2

    # Check if branch exists
    if git show-ref --verify --quiet "refs/heads/$BRANCH_NAME"; then
        # Branch exists, create worktree for it
        git worktree add "$WORKTREE_PATH" "$BRANCH_NAME" >&2
    else
        # Create new branch from main/master
        local base_branch
        base_branch=$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@')
        if [[ -z "$base_branch" ]]; then
            # Fallback: try to detect main or master
            if git show-ref --verify --quiet refs/heads/main; then
                base_branch="main"
            elif git show-ref --verify --quiet refs/heads/master; then
                base_branch="master"
            else
                # Use current branch as fallback
                base_branch=$(git rev-parse --abbrev-ref HEAD)
            fi
        fi
        git worktree add "$WORKTREE_PATH" -b "$BRANCH_NAME" "$base_branch" >&2
    fi

    print_success "Worktree created: $WORKTREE_PATH" >&2
    echo "$WORKTREE_PATH"
}

cleanup_worktree() {
    if [[ "$USE_WORKTREE" != true ]]; then
        return
    fi

    print_info "Cleaning up worktree..."

    # Go back to repo root
    cd "$REPO_ROOT"

    # Remove worktree
    if [[ -d "$WORKTREE_PATH" ]]; then
        git worktree remove "$WORKTREE_PATH" --force 2>/dev/null || true
    fi

    # Optionally delete branch (commented out - user may want to keep it)
    # git branch -D "$BRANCH_NAME" 2>/dev/null || true

    print_success "Worktree removed"
}

merge_to_main() {
    if [[ "$USE_WORKTREE" != true ]]; then
        return
    fi

    print_header "Merging $BRANCH_NAME to main"

    cd "$REPO_ROOT"

    # Get current main branch name
    local main_branch
    main_branch=$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo "main")

    git checkout "$main_branch"
    git pull origin "$main_branch"

    # Attempt merge
    if git merge "$BRANCH_NAME" --no-edit; then
        print_success "Merged successfully"
        git push origin "$main_branch"
        cleanup_worktree
        git branch -d "$BRANCH_NAME" 2>/dev/null || true
    else
        print_error "Merge conflict! Resolve manually:"
        echo "  cd $REPO_ROOT"
        echo "  # Edit conflicted files"
        echo "  git add ."
        echo "  git commit"
        echo "  git push origin $main_branch"
        echo "  git worktree remove $WORKTREE_PATH"
        echo "  git branch -d $BRANCH_NAME"
        exit 1
    fi
}

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# The prompt (generic for any spec)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

generate_prompt() {
    local spec_path="$1"

    cat <<EOF
Read the specification at: $spec_path

DELIVERABLE WORKFLOW (complete ONE deliverable per LLM context):

UNDERSTANDING DELIVERABLES:
- A "deliverable" is a major section in the spec (e.g., "## Bug 1", "## Feature: Auth")
- Each deliverable contains multiple checklist items (checkboxes)
- ONE deliverable = ONE iteration of this loop
- Complete ALL checkboxes within a deliverable before marking it done
- If you see a deliverable with some boxes already checked [x] and others unchecked [ ], you are RESUMING work from a previous agent - pick up where they left off

WORKFLOW:

1. ORIENT
   - Study the spec to understand the project structure
   - Check for CLAUDE.md or AGENTS.md for build/test commands
   - Identify the source code location

2. FIND NEXT DELIVERABLE
   - Scan the spec for deliverable sections (## headers)
   - Find the FIRST deliverable with ANY unchecked [ ] items
   - If a deliverable has SOME items checked [x] and SOME unchecked [ ], resume that one
   - If ALL deliverables are complete (no [ ] items remain), output 'ALL_TASKS_COMPLETE' and exit

3. INVESTIGATE
   - Search the codebase - don't assume something isn't implemented
   - Understand existing patterns before adding new code
   - Read any files mentioned in the deliverable description

4. IMPLEMENT
   - Complete ALL remaining checklist items for this deliverable
   - Write ACTUAL CODE, not just plans or placeholders
   - Follow existing code style and patterns
   - Keep changes focused on the current deliverable

5. VALIDATE
   - Run tests (look for test commands in CLAUDE.md, Makefile, or package.json)
   - Run linter if available
   - Run build to check for warnings
   - Fix any issues before proceeding

6. MARK COMPLETE
   - Update the spec file: change [ ] to [x] for ALL items in this deliverable
   - Only mark complete AFTER code is written AND tests pass
   - Do NOT mark items in other deliverables

7. COMMIT & PUSH
   - Stage relevant files (avoid staging unrelated changes)
   - Commit with descriptive message referencing the deliverable
   - Push to remote: git push origin $BRANCH_NAME

8. SUMMARIZE & EXIT
   - Output a single line: SUMMARY: <brief description of what you did>
   - Then output 'DELIVERABLE_COMPLETE'
   - Exit immediately (the loop will restart for the next deliverable)

CRITICAL RULES:
- ONE deliverable per run (but ALL checkboxes within that deliverable)
- Write REAL CODE, not placeholders
- Do NOT mark [x] until code is written AND validated
- If tests fail, fix them before marking complete
- If stuck, output 'BLOCKED: <reason>' and exit
- If resuming a partial deliverable, acknowledge what's already done and continue
EOF
}

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Run one iteration
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

run_iteration() {
    local work_dir="$1"
    local iteration="$2"

    echo ""
    echo -e "${BLUE}━━━━━━━━━━ Iteration #$iteration [$(date +%H:%M:%S)] ━━━━━━━━━━${NC}"
    echo -e "${CYAN}Spec:${NC} $SPEC_NAME"
    echo -e "${CYAN}Dir:${NC}  $work_dir"
    echo ""

    cd "$work_dir"

    local prompt
    prompt=$(generate_prompt "$SPEC_FILE_ABS")

    local output
    output=$(claude --dangerously-skip-permissions -p "$prompt" 2>&1) || true

    # Extract summary if present
    local summary=""
    if echo "$output" | grep -q "SUMMARY:"; then
        summary=$(echo "$output" | grep "SUMMARY:" | head -1 | sed 's/.*SUMMARY: *//')
    fi

    # Check output for completion markers
    if echo "$output" | grep -q "ALL_TASKS_COMPLETE"; then
        print_success "All tasks complete!"
        return 0  # Signal completion
    fi

    if echo "$output" | grep -q "DELIVERABLE_COMPLETE"; then
        print_success "Deliverable completed"
        if [[ -n "$summary" ]]; then
            echo -e "   ${CYAN}→ $summary${NC}"
        fi
        return 1  # Signal continue
    fi

    if echo "$output" | grep -q "BLOCKED:"; then
        local reason
        reason=$(echo "$output" | grep "BLOCKED:" | head -1)
        print_warning "$reason"
        return 2  # Signal blocked
    fi

    print_warning "No completion marker found"
    if [[ -n "$summary" ]]; then
        echo -e "   ${CYAN}→ $summary${NC}"
    fi
    return 1  # Continue anyway
}

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Main
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

main() {
    print_header "Ralph Loop: $SPEC_NAME"
    echo -e "${CYAN}Spec:${NC}      $SPEC_FILE_ABS"
    echo -e "${CYAN}Mode:${NC}      $(if $LOOP_MODE; then echo "Loop until complete"; else echo "Single iteration (--once)"; fi)"
    echo -e "${CYAN}Worktree:${NC}  $(if $USE_WORKTREE; then echo "Yes ($WORKTREE_PATH)"; else echo "No (current dir)"; fi)"

    # Setup worktree (or use current dir)
    local work_dir
    work_dir=$(setup_worktree)

    local iteration=0
    local retry_count=0

    while true; do
        iteration=$((iteration + 1))

        run_iteration "$work_dir" "$iteration"
        local result=$?

        case $result in
            0)  # All complete
                if [[ "$USE_WORKTREE" == true ]]; then
                    merge_to_main
                fi
                print_header "🎉 SPEC COMPLETE: $SPEC_NAME"
                exit 0
                ;;
            1)  # Continue
                retry_count=0
                ;;
            2)  # Blocked
                retry_count=$((retry_count + 1))
                if [[ $retry_count -ge $MAX_RETRIES ]]; then
                    print_error "Blocked $MAX_RETRIES times. Manual intervention needed."
                    exit 1
                fi
                ;;
        esac

        # If --once mode, exit after one iteration
        if [[ "$LOOP_MODE" != true ]]; then
            print_info "Single iteration complete (--once mode)."
            exit 0
        fi

        echo ""
        echo -e "${YELLOW}⏳ Next iteration in ${DELAY}s...${NC}"
        sleep $DELAY
    done
}

# Trap to cleanup on exit
trap 'echo ""; print_warning "Interrupted. Worktree preserved at: $WORKTREE_PATH"' INT TERM

main
