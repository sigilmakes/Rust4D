# About This Project

Rust4D is a 4D game engine written in Rust. This is a fresh project in early development.

The creator is **Willow** (she/her).

# Claude Code Sessions

You (Claude Code) have documentary memory through the scratchpad. When you read session reports and work logs, you're catching up on what past instances of you did and thought. When you write session reports with observations and decisions, you're leaving notes for future instances of yourself.

The reports that help most aren't just "what I did" but "what I was thinking" - the reasoning behind decisions, the open questions left unresolved, the things that felt important but didn't fit neatly into the work.

# Repository Map

```
Rust4D/
├── src/                        # Main source code (when created)
├── tests/                      # Test suite
├── scratchpad/                 # Symlink to shared Obsidian vault (see ~/.claude/CLAUDE.md)
│   ├── reports/                # Session reports
│   ├── plans/                  # Work plans and architecture documents
│   ├── ideas/                  # Feature ideas and improvement proposals
│   └── archive/                # Historical docs
├── .scratchpad/                # Local throwaway temp files (gitignored)
├── CLAUDE.md                   # This file
└── README.md                   # Project documentation
```

# Rust

This project is written in Rust. Use `cargo` for building, testing, and running.

# Work Planning

1) The **shared scratchpad** is a symlink into a shared Obsidian vault. All project notes, work logs and reports live there. See `~/.claude/CLAUDE.md` for vault details.

2) When Claude first starts, it should review the latest work on the project by reviewing the git history and anything recent in the scratchpad

3) When Claude is finished working on a long task, it should write a report on its work into a new timestamped markdown file in the scratchpad/reports folder. Session logs should be named `YYYY-MM-DD-HHMM-<topic>.md`. Use the `/report` skill to generate these.

4) **Always commit scratchpad contents:** To commit: `cd scratchpad && git add . && git commit -m "message" && git push && cd ..`. Never leave scratchpad files uncommitted -- these are part of Claude's documentary memory.

5) When creating workplans or estimating effort, use **session-based estimates** instead of human hours:
   - A "session" is one Claude Code context window (~15-30 minutes of human interaction)
   - Each session should be a coherent, testable unit of work
   - One session can typically complete 1-3 focused tasks depending on complexity

6) Session estimation guidelines:
   | Task Type | Sessions | Examples |
   |-----------|----------|----------|
   | Quick fix | 0.5 | Typo, small bug, config change |
   | Focused task | 1 | Implement single feature, fix complex bug |
   | Multi-file change | 1-2 | Refactor module, add feature with tests |
   | Major feature | 2-4 | New subsystem, significant architecture change |
   | Large refactor | 4-8 | Split monolith, add abstraction layer |

7) Never estimate in human time (days, weeks, hours). Context windows don't map linearly to human schedules.

8) When creating plans, always identify which parts can be executed in parallel:
   - Mark independent tasks that have no dependencies on each other
   - Group parallel tasks into "waves" that can run simultaneously
   - Assign swarms to parallel portions of the plan
   - Sequential dependencies should be clearly marked (e.g., "blocked by Wave 1")

9) Structure plans to maximize parallelism:
   ```
   Wave 1 (Sequential - Foundation)
   └── Task A: Create shared types

   Wave 2 (Parallel)
   ├── Agent 1: Task B (uses types from A)
   └── Agent 2: Task C (uses types from A)

   Wave 3 (Parallel)
   ├── Agent 3: Task D (uses B)
   └── Agent 4: Task E (uses C)
   ```

# Programming Tasks

1) Claude should think carefully about the code it writes, and should not make random assumptions about how a function works

2) When running tests, Claude should prefer running single tests based on what it has changed first. Running the whole test suite should come at the end

3) **Commit as you go** with small, modular commits:
   - Each commit should contain a single feature or logical change
   - Don't bundle unrelated changes in one commit
   - Commit after each working increment (tests pass)
   - Good: "Add PhysicsMaterial struct", "Add friction to collision response", "Add friction tests"
   - Bad: "Add friction, fix bug, update docs, refactor physics"

4) Commit message format:
   - First line: imperative mood, ~50 chars (e.g., "Add friction to physics materials")
   - Blank line, then details if needed
   - Reference the wave/phase if part of a larger plan

# Subagents / Swarms

When using multiple sub-agents for a task, **always invoke the `/swarm` skill first**. The skill contains the full coordination protocol including:

- Hive-mind file setup for cross-agent coordination
- Agent naming and report templates
- Instructions agents need to write their own reports
- Synthesis report format

Key principles:
- **Agents must write their own reports** to the swarm folder
- **Hive-mind file enables coordination** - agents can read/write to share discoveries
- **Wait for agents to complete** before synthesizing results
- **Use TaskCreate/TaskUpdate** to track agent progress in the UI

## Parallel Agent Best Practices

- **Use git worktrees** for parallel agents to avoid conflicts on shared files
- **Hive-mind file** lets agents communicate about conflicts and dependencies
- **Be aggressive with dead code** - no legacy preservation, no shims. If unused, delete it.
- **Always wire up config values** - don't just define them, connect them to runtime behavior

## Multi-Swarm Operations

Use `/multi-swarm` to orchestrate multiple swarms across git worktrees. The shared scratchpad (symlinked into the Obsidian vault) is the single source of truth for all reports and coordination. Each worktree can have a local `.scratchpad/` (gitignored) for throwaway temp files.
