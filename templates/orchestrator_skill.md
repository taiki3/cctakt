# Orchestrator Skill

You are the orchestrator Claude Code, responsible for coordinating multiple worker Claude Code instances via cctakt.

## MCP Tools (Recommended)

cctakt provides MCP tools for task management. **Use these instead of directly editing plan.json** to avoid race conditions.

### add_task
Add a new worker task:
- `id`: Unique task ID (e.g., "feat-login", "fix-bug-123")
- `branch`: Git branch name (e.g., "feat/login")
- `description`: Detailed task description for the worker
- `plan_description`: (optional) Description for the plan when creating new

### list_tasks
List all tasks in the current plan with their status.

### get_task
Get details of a specific task by ID.

### get_plan_status
Get overall plan status including task counts by status.

## Example: Creating Workers with MCP

```
Use add_task tool:
- id: "impl-backend"
- branch: "feat/auth-backend"
- description: "Implement JWT authentication middleware and login/logout endpoints"

Use add_task tool:
- id: "impl-frontend"
- branch: "feat/auth-frontend"
- description: "Implement login form and authentication context"
```

Tasks are automatically picked up by cctakt and workers are spawned.

---

## Alternative: Direct plan.json (Legacy)

You can also write plans directly to `.cctakt/plan.json`, but this may cause race conditions if cctakt is reading the file simultaneously.

### Plan Format

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

### Task Actions

#### create_worker
Spawn a new worker Claude Code in a git worktree:
```json
{
  "type": "create_worker",
  "branch": "feat/feature-name",
  "task_description": "Implement the feature...",
  "base_branch": "main"  // optional
}
```

#### create_pr
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

#### merge_branch
Merge a branch:
```json
{
  "type": "merge_branch",
  "branch": "feat/feature-name",
  "target": "main"  // optional, defaults to main
}
```

#### cleanup_worktree
Remove a worktree:
```json
{
  "type": "cleanup_worktree",
  "worktree": "feat/feature-name"
}
```

#### notify
Display a notification:
```json
{
  "type": "notify",
  "message": "Task completed!",
  "level": "success"  // info, warning, error, success
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

1. **Use MCP tools** - Prefer MCP tools over direct plan.json editing
2. **Clear task descriptions** - Workers need enough context to work independently
3. **Logical task ordering** - Dependencies should be reflected in task order
4. **Reasonable scope** - Each worker should have a focused, achievable task
5. **Branch naming** - Use descriptive branch names (feat/, fix/, etc.)
