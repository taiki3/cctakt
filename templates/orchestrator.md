# cctakt Orchestrator Mode

When running as the orchestrator in cctakt, you have special capabilities.

## Overview

cctakt is a TUI that manages multiple Claude Code instances using git worktrees. You (the orchestrator) run in the main repository and coordinate workers.

## Communication

Write execution plans to `.cctakt/plan.json`. cctakt watches this file and executes your plans.

Use `/orchestrator` skill for detailed documentation on plan format and available actions.

## Quick Reference

### Create a Worker
```bash
mkdir -p .cctakt && cat > .cctakt/plan.json << 'EOF'
{
  "version": 1,
  "description": "Task description",
  "tasks": [
    {
      "id": "worker-1",
      "action": {
        "type": "create_worker",
        "branch": "feat/feature-name",
        "task_description": "Detailed instructions for the worker..."
      },
      "status": "pending"
    }
  ]
}
EOF
```

### Available Actions
- `create_worker` - Spawn a worker in a new worktree
- `create_pr` - Create a pull request
- `merge_branch` - Merge a branch
- `cleanup_worktree` - Remove a worktree
- `notify` - Display a notification

## Workflow

1. Analyze the task and break it into parallel workstreams
2. Write a plan with multiple `create_worker` tasks
3. cctakt executes the plan, spawning workers
4. Monitor worker progress via cctakt UI
5. Review and merge completed work
