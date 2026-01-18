//! MCP (Model Context Protocol) server for cctakt
//!
//! Provides tools for the orchestrator to manage tasks without directly
//! modifying plan.json, avoiding race conditions.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use crate::plan::{Plan, PlanManager, Task, TaskAction, TaskStatus};

/// JSON-RPC request
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

/// JSON-RPC response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// MCP Tool definition
#[derive(Debug, Serialize)]
struct Tool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

/// MCP Server
pub struct McpServer {
    plan_manager: PlanManager,
}

impl McpServer {
    pub fn new() -> Result<Self> {
        let plan_manager = PlanManager::new(std::path::PathBuf::from("."));
        Ok(Self { plan_manager })
    }

    /// Run the MCP server (stdio mode)
    pub fn run(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = line.context("Failed to read line from stdin")?;
            if line.is_empty() {
                continue;
            }

            let response = self.handle_request(&line);
            let response_json = serde_json::to_string(&response)?;
            writeln!(stdout, "{}", response_json)?;
            stdout.flush()?;
        }

        Ok(())
    }

    fn handle_request(&mut self, line: &str) -> JsonRpcResponse {
        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
            }
        };

        let id = request.id.clone().unwrap_or(Value::Null);

        match request.method.as_str() {
            "initialize" => self.handle_initialize(id, request.params),
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tools_call(id, request.params),
            "notifications/initialized" => {
                // Ignore notification
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(Value::Null),
                    error: None,
                }
            }
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            },
        }
    }

    fn handle_initialize(&self, id: Value, _params: Value) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "cctakt",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            error: None,
        }
    }

    fn handle_tools_list(&self, id: Value) -> JsonRpcResponse {
        let tools = vec![
            Tool {
                name: "add_task".to_string(),
                description: "Add a new worker task to the current plan. Creates a new plan if none exists.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Unique task ID (e.g., 'feat-login', 'fix-bug-123')"
                        },
                        "branch": {
                            "type": "string",
                            "description": "Git branch name for the worker (e.g., 'feat/login', 'fix/bug-123')"
                        },
                        "description": {
                            "type": "string",
                            "description": "Detailed task description for the worker. Include requirements, files to modify, and completion criteria."
                        },
                        "plan_description": {
                            "type": "string",
                            "description": "Optional: Description for the plan (only used when creating a new plan)"
                        }
                    },
                    "required": ["id", "branch", "description"]
                }),
            },
            Tool {
                name: "list_tasks".to_string(),
                description: "List all tasks in the current plan with their status.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            Tool {
                name: "get_task".to_string(),
                description: "Get details of a specific task by ID.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Task ID to look up"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "get_plan_status".to_string(),
                description: "Get overall plan status including task counts by status.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        ];

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({ "tools": tools })),
            error: None,
        }
    }

    fn handle_tools_call(&mut self, id: Value, params: Value) -> JsonRpcResponse {
        let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        let result = match name {
            "add_task" => self.tool_add_task(arguments),
            "list_tasks" => self.tool_list_tasks(),
            "get_task" => self.tool_get_task(arguments),
            "get_plan_status" => self.tool_get_plan_status(),
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        };

        match result {
            Ok(content) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": content
                    }]
                })),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Error: {}", e)
                    }],
                    "isError": true
                })),
                error: None,
            },
        }
    }

    fn tool_add_task(&mut self, args: Value) -> Result<String> {
        let task_id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;
        let branch = args
            .get("branch")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: branch"))?;
        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: description"))?;
        let plan_description = args.get("plan_description").and_then(|v| v.as_str());

        // Load or create plan
        let mut plan = self.plan_manager.load()?.unwrap_or_else(|| {
            match plan_description {
                Some(desc) => Plan::with_description(desc),
                None => Plan::new(),
            }
        });

        // Check if task already exists
        if plan.get_task(task_id).is_some() {
            return Err(anyhow::anyhow!("Task with ID '{}' already exists", task_id));
        }

        // Add task
        let task = Task::new(
            task_id.to_string(),
            TaskAction::CreateWorker {
                branch: branch.to_string(),
                task_description: description.to_string(),
                base_branch: None,
            },
        );
        plan.add_task(task);

        // Save plan
        self.plan_manager.save(&plan)?;

        Ok(format!(
            "Task '{}' added successfully.\n\nBranch: {}\nStatus: pending\n\nThe task will be picked up by cctakt automatically.",
            task_id, branch
        ))
    }

    fn tool_list_tasks(&mut self) -> Result<String> {
        let plan = self.plan_manager.load()?;

        match plan {
            Some(plan) => {
                if plan.tasks.is_empty() {
                    return Ok("No tasks in current plan.".to_string());
                }

                let mut output = String::new();
                if let Some(ref desc) = plan.description {
                    output.push_str(&format!("Plan: {}\n\n", desc));
                }
                output.push_str("Tasks:\n");

                for task in &plan.tasks {
                    let status_emoji = match task.status {
                        TaskStatus::Pending => "⏳",
                        TaskStatus::Running => "🔄",
                        TaskStatus::Completed => "✅",
                        TaskStatus::Failed => "❌",
                        TaskStatus::Skipped => "⏭️",
                    };
                    output.push_str(&format!(
                        "  {} {} - {:?}\n",
                        status_emoji, task.id, task.status
                    ));

                    if let TaskAction::CreateWorker { branch, .. } = &task.action {
                        output.push_str(&format!("      Branch: {}\n", branch));
                    }
                }

                Ok(output)
            }
            None => Ok("No active plan. Use add_task to create one.".to_string()),
        }
    }

    fn tool_get_task(&mut self, args: Value) -> Result<String> {
        let task_id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

        let plan = self.plan_manager.load()?;

        match plan {
            Some(plan) => {
                if let Some(task) = plan.get_task(task_id) {
                    let mut output = format!("Task: {}\n", task.id);
                    output.push_str(&format!("Status: {:?}\n", task.status));

                    if let TaskAction::CreateWorker {
                        branch,
                        task_description,
                        ..
                    } = &task.action
                    {
                        output.push_str(&format!("Branch: {}\n", branch));
                        output.push_str(&format!("\nDescription:\n{}\n", task_description));
                    }

                    if let Some(ref result) = task.result {
                        output.push_str(&format!("\nResult:\n"));
                        if !result.commits.is_empty() {
                            output.push_str("  Commits:\n");
                            for commit in &result.commits {
                                output.push_str(&format!("    - {}\n", commit));
                            }
                        }
                        if let Some(ref url) = result.pr_url {
                            output.push_str(&format!("  PR: {}\n", url));
                        }
                    }

                    if let Some(ref error) = task.error {
                        output.push_str(&format!("\nError: {}\n", error));
                    }

                    Ok(output)
                } else {
                    Err(anyhow::anyhow!("Task '{}' not found", task_id))
                }
            }
            None => Err(anyhow::anyhow!("No active plan")),
        }
    }

    fn tool_get_plan_status(&mut self) -> Result<String> {
        let plan = self.plan_manager.load()?;

        match plan {
            Some(plan) => {
                let (pending, running, completed, failed) = plan.count_by_status();
                let total = plan.tasks.len();

                let mut output = String::new();
                if let Some(ref desc) = plan.description {
                    output.push_str(&format!("Plan: {}\n\n", desc));
                }

                output.push_str(&format!("Total tasks: {}\n", total));
                output.push_str(&format!("  ⏳ Pending:   {}\n", pending));
                output.push_str(&format!("  🔄 Running:   {}\n", running));
                output.push_str(&format!("  ✅ Completed: {}\n", completed));
                output.push_str(&format!("  ❌ Failed:    {}\n", failed));

                if plan.is_complete() {
                    output.push_str("\n✨ All tasks completed!");
                }

                Ok(output)
            }
            None => Ok("No active plan.".to_string()),
        }
    }
}
