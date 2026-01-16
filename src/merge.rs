//! Merge manager module for cctakt
//!
//! Provides functionality for merging completed agent branches
//! back into the main branch.

use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::process::Command;

/// Preview information for a merge operation
#[derive(Debug, Clone)]
pub struct MergePreview {
    /// The branch being merged
    pub branch: String,
    /// Number of files changed
    pub files_changed: usize,
    /// Number of insertions
    pub insertions: usize,
    /// Number of deletions
    pub deletions: usize,
    /// List of files with potential conflicts
    pub conflicts: Vec<String>,
}

/// Manager for git merge operations
///
/// # Example
/// ```ignore
/// let merger = MergeManager::new("/path/to/repo");
///
/// // Preview the merge
/// let preview = merger.preview("feat/auth")?;
/// println!("Files changed: {}", preview.files_changed);
///
/// // Get the diff
/// let diff = merger.diff("feat/auth")?;
///
/// // Perform the merge
/// merger.merge("feat/auth", Some("Merge feature branch"))?;
/// ```
pub struct MergeManager {
    repo_path: PathBuf,
    main_branch: String,
}

impl MergeManager {
    /// Create a new merge manager for the given repository
    pub fn new(repo_path: impl Into<PathBuf>) -> Self {
        Self {
            repo_path: repo_path.into(),
            main_branch: "main".to_string(),
        }
    }

    /// Set the main branch name (default: "main")
    pub fn with_main_branch(mut self, branch: impl Into<String>) -> Self {
        self.main_branch = branch.into();
        self
    }

    /// Get the configured main branch name
    pub fn main_branch(&self) -> &str {
        &self.main_branch
    }

    /// Run a git command and return its output
    fn run_git(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to execute git command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Git command failed: {}", stderr.trim());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get a preview of what the merge would look like
    ///
    /// Uses `git diff --stat main...<branch>` to gather statistics.
    pub fn preview(&self, branch: &str) -> Result<MergePreview> {
        // Get diff stats
        let diff_stat = self
            .run_git(&["diff", "--stat", &format!("{}...{}", self.main_branch, branch)])
            .context("Failed to get diff stats")?;

        // Parse the stats
        let (files_changed, insertions, deletions) = parse_diff_stat(&diff_stat);

        // Check for potential conflicts using merge-tree (dry run)
        let conflicts = self.check_conflicts(branch)?;

        Ok(MergePreview {
            branch: branch.to_string(),
            files_changed,
            insertions,
            deletions,
            conflicts,
        })
    }

    /// Check for potential merge conflicts
    fn check_conflicts(&self, branch: &str) -> Result<Vec<String>> {
        // Try to find the merge base
        let merge_base = self.run_git(&["merge-base", &self.main_branch, branch]);

        if merge_base.is_err() {
            // If there's no merge base, branches are unrelated
            return Ok(vec![]);
        }

        // Use merge-tree to simulate the merge (available in newer git versions)
        // Fall back to checking if the same files were modified on both sides
        let files_on_main = self
            .run_git(&["diff", "--name-only", &format!("{}..{}", merge_base.as_ref().unwrap().trim(), &self.main_branch)])
            .unwrap_or_default();

        let files_on_branch = self
            .run_git(&["diff", "--name-only", &format!("{}..{}", merge_base.as_ref().unwrap().trim(), branch)])
            .unwrap_or_default();

        let main_files: std::collections::HashSet<_> = files_on_main.lines().collect();
        let branch_files: std::collections::HashSet<_> = files_on_branch.lines().collect();

        // Files modified on both sides could have conflicts
        let potential_conflicts: Vec<String> = main_files
            .intersection(&branch_files)
            .map(|s| s.to_string())
            .collect();

        Ok(potential_conflicts)
    }

    /// Get the full diff between main and the branch
    ///
    /// Uses `git diff main...<branch>`.
    pub fn diff(&self, branch: &str) -> Result<String> {
        self.run_git(&["diff", &format!("{}...{}", self.main_branch, branch)])
            .context("Failed to get diff")
    }

    /// Get a file-specific diff
    pub fn diff_file(&self, branch: &str, file: &str) -> Result<String> {
        self.run_git(&["diff", &format!("{}...{}", self.main_branch, branch), "--", file])
            .context("Failed to get file diff")
    }

    /// Perform the merge
    ///
    /// Uses `git merge <branch> -m "<message>"`.
    pub fn merge(&self, branch: &str, message: Option<&str>) -> Result<()> {
        let default_message = format!("Merge branch '{}' into {}", branch, self.main_branch);
        let msg = message.unwrap_or(&default_message);

        self.run_git(&["merge", branch, "-m", msg])
            .context("Failed to merge branch")?;

        Ok(())
    }

    /// Perform a merge without fast-forward
    pub fn merge_no_ff(&self, branch: &str, message: Option<&str>) -> Result<()> {
        let default_message = format!("Merge branch '{}' into {}", branch, self.main_branch);
        let msg = message.unwrap_or(&default_message);

        self.run_git(&["merge", "--no-ff", branch, "-m", msg])
            .context("Failed to merge branch")?;

        Ok(())
    }

    /// Abort an in-progress merge
    ///
    /// Uses `git merge --abort`.
    pub fn abort(&self) -> Result<()> {
        self.run_git(&["merge", "--abort"])
            .context("Failed to abort merge")?;

        Ok(())
    }

    /// Delete a branch
    ///
    /// Uses `git branch -d <branch>` (safe delete, fails if not merged).
    pub fn delete_branch(&self, branch: &str) -> Result<()> {
        self.run_git(&["branch", "-d", branch])
            .context("Failed to delete branch")?;

        Ok(())
    }

    /// Force delete a branch
    ///
    /// Uses `git branch -D <branch>` (force delete).
    pub fn force_delete_branch(&self, branch: &str) -> Result<()> {
        self.run_git(&["branch", "-D", branch])
            .context("Failed to force delete branch")?;

        Ok(())
    }

    /// Get list of branches
    pub fn list_branches(&self) -> Result<Vec<String>> {
        let output = self.run_git(&["branch", "--list", "--format=%(refname:short)"])?;
        Ok(output.lines().map(|s| s.to_string()).collect())
    }

    /// Get the current branch name
    pub fn current_branch(&self) -> Result<String> {
        let output = self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        Ok(output.trim().to_string())
    }

    /// Check if a branch exists
    pub fn branch_exists(&self, branch: &str) -> bool {
        self.run_git(&["rev-parse", "--verify", branch]).is_ok()
    }

    /// Checkout a branch
    pub fn checkout(&self, branch: &str) -> Result<()> {
        self.run_git(&["checkout", branch])
            .context("Failed to checkout branch")?;
        Ok(())
    }
}

/// Parse the output of `git diff --stat` to extract statistics
fn parse_diff_stat(stat: &str) -> (usize, usize, usize) {
    let mut files_changed = 0;
    let mut insertions = 0;
    let mut deletions = 0;

    // Look for the summary line: " X files changed, Y insertions(+), Z deletions(-)"
    for line in stat.lines() {
        let line = line.trim();

        // Summary line format: " N file(s) changed, X insertion(s)(+), Y deletion(s)(-)"
        if line.contains("changed") {
            // Parse files changed
            if let Some(files_str) = line.split_whitespace().next() {
                files_changed = files_str.parse().unwrap_or(0);
            }

            // Parse insertions
            if let Some(pos) = line.find("insertion") {
                let before = &line[..pos];
                if let Some(num_str) = before.split(',').last() {
                    let num_str = num_str.trim();
                    insertions = num_str.parse().unwrap_or(0);
                }
            }

            // Parse deletions
            if let Some(pos) = line.find("deletion") {
                let before = &line[..pos];
                if let Some(num_str) = before.split(',').last() {
                    let num_str = num_str.trim();
                    deletions = num_str.parse().unwrap_or(0);
                }
            }
        }
    }

    (files_changed, insertions, deletions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_manager_new() {
        let manager = MergeManager::new("/tmp/test-repo");
        assert_eq!(manager.repo_path, PathBuf::from("/tmp/test-repo"));
        assert_eq!(manager.main_branch, "main");
    }

    #[test]
    fn test_merge_manager_with_main_branch() {
        let manager = MergeManager::new("/tmp/test-repo").with_main_branch("master");
        assert_eq!(manager.main_branch(), "master");
    }

    #[test]
    fn test_parse_diff_stat_full() {
        let stat = r#"
 src/main.rs | 10 +++++-----
 src/lib.rs  | 20 ++++++++++++++++++++
 2 files changed, 25 insertions(+), 5 deletions(-)
"#;
        let (files, ins, del) = parse_diff_stat(stat);
        assert_eq!(files, 2);
        assert_eq!(ins, 25);
        assert_eq!(del, 5);
    }

    #[test]
    fn test_parse_diff_stat_insertions_only() {
        let stat = r#"
 src/new.rs | 50 ++++++++++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 50 insertions(+)
"#;
        let (files, ins, del) = parse_diff_stat(stat);
        assert_eq!(files, 1);
        assert_eq!(ins, 50);
        assert_eq!(del, 0);
    }

    #[test]
    fn test_parse_diff_stat_deletions_only() {
        let stat = r#"
 src/old.rs | 30 ------------------------------
 1 file changed, 30 deletions(-)
"#;
        let (files, ins, del) = parse_diff_stat(stat);
        assert_eq!(files, 1);
        assert_eq!(ins, 0);
        assert_eq!(del, 30);
    }

    #[test]
    fn test_parse_diff_stat_empty() {
        let stat = "";
        let (files, ins, del) = parse_diff_stat(stat);
        assert_eq!(files, 0);
        assert_eq!(ins, 0);
        assert_eq!(del, 0);
    }

    #[test]
    fn test_merge_preview_new() {
        let preview = MergePreview {
            branch: "feat/test".to_string(),
            files_changed: 5,
            insertions: 100,
            deletions: 20,
            conflicts: vec!["src/main.rs".to_string()],
        };

        assert_eq!(preview.branch, "feat/test");
        assert_eq!(preview.files_changed, 5);
        assert_eq!(preview.insertions, 100);
        assert_eq!(preview.deletions, 20);
        assert_eq!(preview.conflicts.len(), 1);
    }
}
