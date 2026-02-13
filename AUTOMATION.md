# Phase Automation Script

This script automatically implements the CLI specification phase by phase.

## Usage

```bash
# Run from project root
./phase-automation.sh
```

## What It Does

1. **Reads the spec** - `specs/command-line-interface.md`
2. **Finds next unchecked item** - First `[ ]` deliverable
3. **Creates phase branch** - E.g., `phase-5-context-tracking`
4. **Implements the code** - Writes actual code in `src/` or `crates/`
5. **Writes tests** - Adds unit/integration tests
6. **Runs tests** - `cargo test --package coding-agent-cli`
7. **Verifies build** - `cargo build --package coding-agent-cli`
8. **Marks complete** - Changes `[ ]` to `[x]` in spec
9. **Commits changes** - With descriptive message
10. **Merges when done** - Merges phase branch to main when complete
11. **Repeats** - Moves to next deliverable

## Features

- ✅ **Automatic retry** - Retries up to 3 times on failure
- ✅ **Color-coded output** - Green (success), Yellow (warning), Red (error)
- ✅ **Progress tracking** - Shows iteration count, branch, retry status
- ✅ **Smart stopping** - Exits when all phases complete or on persistent errors
- ✅ **Git integration** - Manages branches, commits, merges automatically
- ✅ **Test validation** - Won't mark complete until tests pass

## Configuration

Edit these variables at the top of the script:

```bash
DELAY=3           # Seconds between iterations
MAX_RETRIES=3     # Max consecutive failures before stopping
```

## Output

The script provides clear, color-coded feedback:

```
━━━━━━━━━━ Iteration #5 [10:30:15] ━━━━━━━━━━
🚀 Starting next deliverable...

✅ Deliverable completed successfully

📊 Current state:
   Branch: phase-5-context-tracking
   Iteration: #5
   Retry count: 0/3

⏳ Waiting 3s before next iteration...
```

## Stopping the Script

- **Automatic stop** - When all phases complete
- **Manual stop** - Press `Ctrl+C`
- **Error stop** - After 3 consecutive failures

## Resuming

The script automatically picks up where it left off by reading the spec file. Just run it again:

```bash
./phase-automation.sh
```

## Monitoring Progress

Check the spec file at any time to see progress:

```bash
grep -E '\[[ x]\]' specs/command-line-interface.md | head -20
```

Or check git commits:

```bash
git log --oneline --graph --all | head -20
```

## Troubleshooting

### Script fails repeatedly

1. Check the last output: Look at the error messages
2. Run tests manually: `cargo test --package coding-agent-cli`
3. Check git status: `git status`
4. Fix issues manually, then restart the script

### Branch conflicts

If you have uncommitted changes:

```bash
git stash
./phase-automation.sh
# Later: git stash pop
```

### Need to restart a phase

```bash
# Delete the phase branch
git branch -D phase-N-name

# Reset spec changes
git checkout specs/command-line-interface.md

# Restart script
./phase-automation.sh
```

## Safety

- ✅ Works in sandbox mode (uses `--dangerously-skip-permissions`)
- ✅ Each deliverable is isolated in commits
- ✅ Tests must pass before marking complete
- ✅ Build must succeed before proceeding
- ✅ Can be interrupted and resumed safely

## Example Run

```
🤖 Phase Automation Started
📋 Spec: /Users/charliposner/coding-agent/specs/command-line-interface.md
⏱️  Delay: 3s between iterations
🔄 Max retries: 3

━━━━━━━━━━ Iteration #1 [10:15:30] ━━━━━━━━━━
🚀 Starting next deliverable...

✅ Deliverable completed successfully

📊 Current state:
   Branch: phase-5-context-tracking
   Iteration: #1
   Retry count: 0/3

⏳ Waiting 3s before next iteration...

━━━━━━━━━━ Iteration #2 [10:15:45] ━━━━━━━━━━
🚀 Starting next deliverable...

✅ Deliverable completed successfully
...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎉 ALL PHASES COMPLETE!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ All deliverables implemented and tested
✅ Automation finished at Fri Feb 14 11:30:00 PST 2026
```
