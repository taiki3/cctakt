//! Run command implementation (CLI mode)

use crate::git_utils::get_worker_commits;
use anyhow::{Context, Result};
use cctakt::{Config, Plan, TaskAction, TaskResult, TaskStatus, WorktreeManager};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Run workers from a plan file (CLI mode)
pub fn run_plan(plan_path: PathBuf) -> Result<()> {
    println!("Loading plan from: {}", plan_path.display());

    // Load plan
    let plan_content = fs::read_to_string(&plan_path)
        .with_context(|| format!("Failed to read plan file: {}", plan_path.display()))?;
    let mut plan: Plan =
        serde_json::from_str(&plan_content).with_context(|| "Failed to parse plan JSON")?;

    println!(
        "Plan: {}",
        plan.description.as_deref().unwrap_or("(no description)")
    );
    println!("Tasks: {}", plan.tasks.len());
    println!();

    // Load config for worktree settings
    let config = Config::load().unwrap_or_default();
    let worktree_manager =
        WorktreeManager::from_current_dir().context("Failed to initialize worktree manager")?;

    // Process pending create_worker tasks
    for task in &mut plan.tasks {
        if task.status != TaskStatus::Pending {
            println!("[{}] Skipping (status: {:?})", task.id, task.status);
            continue;
        }

        let TaskAction::CreateWorker {
            branch,
            task_description,
            base_branch: _,
        } = &task.action
        else {
            println!("[{}] Skipping (not a create_worker task)", task.id);
            continue;
        };

        println!("========================================");
        println!("[{}] Starting worker", task.id);
        println!("Branch: {branch}");
        println!("Task: {}", task_description.lines().next().unwrap_or(""));
        println!("========================================");

        // Create worktree
        let worktree_path = match worktree_manager.create(branch, &config.worktree_dir) {
            Ok(path) => {
                println!("Created worktree: {}", path.display());
                path
            }
            Err(e) => {
                println!("Failed to create worktree: {e}");
                task.status = TaskStatus::Failed;
                task.error = Some(format!("Failed to create worktree: {e}"));
                continue;
            }
        };

        // Update task status
        task.status = TaskStatus::Running;

        // Build command
        let mut cmd = Command::new("claude");
        cmd.arg("-p")
            .arg(task_description)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose")
            .arg("--dangerously-skip-permissions");

        cmd.current_dir(&worktree_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        println!("\n--- Worker output ---\n");

        // Spawn process
        let mut child = cmd.spawn().context("Failed to spawn claude")?;

        // Read stdout
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                // Parse and display JSON events
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                    let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match event_type {
                        "system" => {
                            let subtype =
                                json.get("subtype").and_then(|v| v.as_str()).unwrap_or("");
                            println!("[SYS] {subtype}");
                        }
                        "assistant" => {
                            // Extract only text content (skip tool_use)
                            if let Some(content) = json
                                .get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_array())
                            {
                                for block in content {
                                    if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                                        if let Some(text) =
                                            block.get("text").and_then(|t| t.as_str())
                                        {
                                            let preview: String = text.chars().take(100).collect();
                                            if !preview.trim().is_empty() {
                                                println!(
                                                    "[AI] {}...",
                                                    preview.replace('\n', " ")
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        "result" => {
                            let subtype =
                                json.get("subtype").and_then(|v| v.as_str()).unwrap_or("");
                            println!("[RESULT] {subtype}");
                        }
                        _ => {}
                    }
                }
            }
        }

        // Wait for process to finish
        let status = child.wait()?;
        println!("\n--- Worker finished (exit: {status}) ---\n");

        // Get commits
        let commits = get_worker_commits(&worktree_path);
        println!("Commits: {}", commits.len());
        for commit in &commits {
            println!("  - {commit}");
        }

        // Update task
        if status.success() {
            task.status = TaskStatus::Completed;
            task.result = Some(TaskResult {
                commits,
                pr_number: None,
                pr_url: None,
            });
        } else {
            task.status = TaskStatus::Failed;
            task.error = Some(format!("Process exited with: {status}"));
        }

        println!();
    }

    // Save updated plan
    let updated_plan = serde_json::to_string_pretty(&plan)?;
    fs::write(&plan_path, updated_plan)?;
    println!("Plan saved to: {}", plan_path.display());

    Ok(())
}
