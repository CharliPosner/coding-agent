# Ralph - Autonomous Spec Implementation Loop

Ralph is a general-purpose automation script that implements specification files task by task, using Claude to write the actual code.

## Usage

```bash
# Basic usage - loop until spec complete
./ralph.sh specs/my-feature.md

# Run in background
./ralph.sh specs/auth.md &

# Run multiple specs in parallel (each gets its own worktree)
./ralph.sh specs/auth.md &
./ralph.sh specs/api.md &
./ralph.sh specs/ui.md &

# Single iteration only (for debugging)
./ralph.sh specs/auth.md --once

# Run in current directory (no worktree isolation)
./ralph.sh specs/auth.md --no-worktree
```

## What It Does

1. **Creates isolated worktree** - Each spec runs in its own git worktree
2. **Reads the spec** - Finds unchecked items (`[ ]` or `- [ ]`)
3. **Implements one task** - Writes actual code, not just plans
4. **Validates** - Runs tests and build
5. **Marks complete** - Changes `[ ]` to `[x]` in spec
6. **Commits & pushes** - With descriptive message
7. **Repeats** - Loops until all tasks complete
8. **Merges** - Merges branch to main when done

## Features

- **Worktree isolation** - Each spec gets its own working directory
- **Parallel execution** - Run multiple specs simultaneously
- **Automatic retry** - Retries up to 3 times on blocks
- **Color-coded output** - Green (success), Yellow (warning), Red (error)
- **Progress tracking** - Shows iteration count and status
- **Smart completion** - Detects ALL_TASKS_COMPLETE, DELIVERABLE_COMPLETE, BLOCKED

## Configuration

Edit variables at the top of the script:

```bash
WORKTREE_DIR=".worktrees"  # Where worktrees are created
DELAY=3                     # Seconds between iterations
MAX_RETRIES=3               # Max consecutive blocks before stopping
```

## Output

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Ralph Loop: auth
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Spec:      /path/to/specs/auth.md
Mode:      Loop until complete
Worktree:  Yes (/path/to/.worktrees/auth)

━━━━━━━━━━ Iteration #1 [10:30:15] ━━━━━━━━━━
Spec: auth
Dir:  /path/to/.worktrees/auth

✓ Deliverable completed
   → Added POST /api/login endpoint with JWT token generation

⏳ Next iteration in 3s...

━━━━━━━━━━ Iteration #2 [10:32:45] ━━━━━━━━━━
...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎉 SPEC COMPLETE: auth
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## Completion Markers

Claude outputs these markers to signal status:

| Marker | Meaning |
|--------|---------|
| `SUMMARY: <text>` | Brief description of what was done (displayed to user) |
| `ALL_TASKS_COMPLETE` | No unchecked items remain; merge and exit |
| `DELIVERABLE_COMPLETE` | One task done; continue to next |
| `BLOCKED: <reason>` | Cannot proceed; retry or stop |

## Stopping the Script

- **Automatic** - When all tasks complete (merges to main)
- **Manual** - Press `Ctrl+C` (worktree is preserved)
- **Error** - After 3 consecutive BLOCKED outputs

## Resuming

Just run the same command again. Ralph picks up where it left off by reading the spec file:

```bash
./ralph.sh specs/auth.md
```

The worktree and branch are reused if they exist.

## Monitoring Progress

Check spec file progress:
```bash
grep -E '\[[ x]\]' specs/auth.md
```

Check git activity:
```bash
git log --oneline --all | head -20
```

List active worktrees:
```bash
git worktree list
```

## Troubleshooting

### Script fails repeatedly

1. Check the output for error messages
2. Run tests manually in the worktree
3. Fix issues, then restart the script

### Need to reset a spec

```bash
# Remove worktree
git worktree remove .worktrees/auth --force

# Delete the branch
git branch -D ralph-auth

# Reset spec file
git checkout specs/auth.md

# Restart
./ralph.sh specs/auth.md
```

### Merge conflicts

If merge fails, the script prints manual resolution steps:
```bash
cd /path/to/repo
# Edit conflicted files
git add .
git commit
git push origin main
git worktree remove .worktrees/auth
git branch -d ralph-auth
```

## How It Works

1. **Worktree setup** - Creates `.worktrees/<spec-name>/` with branch `ralph-<spec-name>`
2. **Prompt generation** - Sends spec path to Claude with workflow instructions
3. **Iteration loop** - Runs Claude, checks output for markers, handles results
4. **Merge** - On ALL_TASKS_COMPLETE, merges branch to main and cleans up

## Writing Specs for Ralph

Ralph works best with specs that have:

- Clear checkbox items: `- [ ] Implement feature X`
- Specific, actionable tasks (not vague goals)
- Test criteria (so Claude knows when validation passes)
- Build/test commands documented (or in CLAUDE.md)

Example spec format:
```markdown
# Feature: User Authentication

## Deliverables

- [ ] Add login endpoint POST /api/login
- [ ] Add logout endpoint POST /api/logout
- [ ] Add session middleware
- [ ] Add login page UI
- [ ] Add tests for auth flow
```

## Safety

- Uses `--dangerously-skip-permissions` (runs in sandbox mode)
- Each task is isolated in its own commit
- Tests must pass before marking complete
- Can be interrupted and resumed safely
- Worktrees provide isolation from main branch
