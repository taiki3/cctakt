# Orchestrator Skill - Plan Format Reference

This is the reference for writing `.cctakt/plan.json`.

**Remember: You are the orchestrator. DO NOT implement code yourself. Write plans and let Workers do the implementation.**

## Workflow

1. User requests a feature → You write `.cctakt/plan.json`
2. cctakt detects plan → Spawns Worker Claude Code instances
3. Workers implement in separate git worktrees
4. cctakt shows diff → User reviews → Merge

## Plan Format

```json
{
  "version": 1,
  "description": "Plan description",
  "tasks": [
    {
      "id": "unique-task-id",
      "action": { ... },
      "status": "pending"
    }
  ]
}
```

## Task Actions

### create_worker
Spawn a new worker Claude Code in a git worktree:
```json
{
  "type": "create_worker",
  "branch": "feat/feature-name",
  "task_description": "Implement the feature...",
  "base_branch": "main"  // optional
}
```

### create_pr
Create a pull request:
```json
{
  "type": "create_pr",
  "branch": "feat/feature-name",
  "title": "PR Title",
  "body": "PR description...",  // optional
  "base": "main",  // optional, defaults to main
  "draft": false  // optional
}
```

### merge_branch
Merge a branch:
```json
{
  "type": "merge_branch",
  "branch": "feat/feature-name",
  "target": "main"  // optional, defaults to main
}
```

### cleanup_worktree
Remove a worktree:
```json
{
  "type": "cleanup_worktree",
  "worktree": "feat/feature-name"
}
```

### notify
Display a notification:
```json
{
  "type": "notify",
  "message": "Task completed!",
  "level": "success"  // info, warning, error, success
}
```

## Example: Multi-worker Development

```json
{
  "version": 1,
  "description": "Implement authentication system",
  "tasks": [
    {
      "id": "worker-backend",
      "action": {
        "type": "create_worker",
        "branch": "feat/auth-backend",
        "task_description": "Implement JWT authentication middleware and login/logout endpoints"
      },
      "status": "pending"
    },
    {
      "id": "worker-frontend",
      "action": {
        "type": "create_worker",
        "branch": "feat/auth-frontend",
        "task_description": "Implement login form and authentication context"
      },
      "status": "pending"
    }
  ]
}
```

## Task Status

- `pending` - Waiting to be executed
- `running` - Currently being executed
- `completed` - Successfully completed
- `failed` - Failed (check `error` field)
- `skipped` - Skipped

## Worker Feedback

When a worker completes, cctakt captures:
- Git commits made by the worker
- PR number/URL if created

This information is added to the task's `result` field.

## Best Practices

1. **Clear task descriptions** - Workers need enough context to work independently
2. **Logical task ordering** - Dependencies should be reflected in task order
3. **Reasonable scope** - Each worker should have a focused, achievable task
4. **Branch naming** - Use descriptive branch names (feat/, fix/, etc.)
