//! Git Worktree Manager Module
//!
//! Git Worktreeの作成・削除・一覧を管理する独立モジュール。

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Worktreeの情報
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub is_main: bool,
}

/// Git Worktree Manager
pub struct WorktreeManager {
    repo_path: PathBuf,
}

impl WorktreeManager {
    /// リポジトリパスを指定して初期化
    pub fn new(repo_path: impl Into<PathBuf>) -> Result<Self> {
        let repo_path = repo_path.into();

        // gitリポジトリかどうか確認
        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(["rev-parse", "--git-dir"])
            .output()
            .context("Failed to execute git command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Not a git repository: {}",
                repo_path.display()
            ));
        }

        Ok(Self { repo_path })
    }

    /// 現在のディレクトリからリポジトリを検出
    pub fn from_current_dir() -> Result<Self> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .context("Failed to execute git command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Not in a git repository"));
        }

        let repo_path = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        Self::new(PathBuf::from(repo_path))
    }

    /// 新しいWorktreeを作成
    /// - branch: ブランチ名（新規作成される）
    /// - base_dir: Worktreeを作成するベースディレクトリ（例: .worktrees/）
    /// - 戻り値: 作成されたWorktreeのパス
    pub fn create(&self, branch: &str, base_dir: &Path) -> Result<PathBuf> {
        // 1. ブランチ名をサニタイズ
        let safe_branch = sanitize_branch_name(branch);

        // 2. ユニークなブランチ名を確保
        let unique_branch = self.generate_unique_branch(&safe_branch)?;

        // 3. Worktreeのパスを決定（base_dirが相対パスの場合はrepo_pathからの相対）
        let base_path = if base_dir.is_absolute() {
            base_dir.to_path_buf()
        } else {
            self.repo_path.join(base_dir)
        };

        // base_dirが存在しない場合は作成
        if !base_path.exists() {
            std::fs::create_dir_all(&base_path)
                .with_context(|| format!("Failed to create directory: {}", base_path.display()))?;
        }

        let worktree_name = unique_branch.replace('/', "-");
        let worktree_path = base_path.join(&worktree_name);

        // 4. git worktree add -b <branch> <path> を実行
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args([
                "worktree",
                "add",
                "-b",
                &unique_branch,
                worktree_path.to_str().context("Invalid path")?,
            ])
            .output()
            .context("Failed to execute git worktree add")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to create worktree: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(worktree_path)
    }

    /// Worktreeを削除
    pub fn remove(&self, path: &Path) -> Result<()> {
        // 1. git worktree remove --force <path>
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args([
                "worktree",
                "remove",
                "--force",
                path.to_str().context("Invalid path")?,
            ])
            .output()
            .context("Failed to execute git worktree remove")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to remove worktree: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // 2. 親ディレクトリが空なら削除
        if let Some(parent) = path.parent() {
            if parent.exists() {
                if let Ok(mut entries) = parent.read_dir() {
                    if entries.next().is_none() {
                        let _ = std::fs::remove_dir(parent);
                    }
                }
            }
        }

        Ok(())
    }

    /// 全Worktreeを一覧
    pub fn list(&self) -> Result<Vec<WorktreeInfo>> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["worktree", "list", "--porcelain"])
            .output()
            .context("Failed to execute git worktree list")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to list worktrees: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut worktrees = Vec::new();
        let mut current_path: Option<PathBuf> = None;
        let mut current_branch: Option<String> = None;

        for line in stdout.lines() {
            if let Some(path_str) = line.strip_prefix("worktree ") {
                current_path = Some(PathBuf::from(path_str));
            } else if let Some(branch_str) = line.strip_prefix("branch refs/heads/") {
                current_branch = Some(branch_str.to_string());
            } else if line.is_empty() {
                if let Some(path) = current_path.take() {
                    let branch = current_branch.take().unwrap_or_default();
                    let is_main = path == self.repo_path;
                    worktrees.push(WorktreeInfo { path, branch, is_main });
                }
                current_branch = None;
            }
        }

        // 最後のエントリを処理（末尾に空行がない場合）
        if let Some(path) = current_path.take() {
            let branch = current_branch.take().unwrap_or_default();
            let is_main = path == self.repo_path;
            worktrees.push(WorktreeInfo { path, branch, is_main });
        }

        Ok(worktrees)
    }

    /// ブランチが既に存在するかチェック
    pub fn branch_exists(&self, branch: &str) -> Result<bool> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["rev-parse", "--verify", &format!("refs/heads/{}", branch)])
            .output()
            .context("Failed to execute git rev-parse")?;

        Ok(output.status.success())
    }

    /// ユニークなブランチ名を生成
    /// 例: "cctakt/task" -> "cctakt/task", "cctakt/task-2", "cctakt/task-3"
    pub fn generate_unique_branch(&self, base_name: &str) -> Result<String> {
        if !self.branch_exists(base_name)? {
            return Ok(base_name.to_string());
        }

        let mut counter = 2;
        loop {
            let candidate = format!("{}-{}", base_name, counter);
            if !self.branch_exists(&candidate)? {
                return Ok(candidate);
            }
            counter += 1;

            // 無限ループ防止
            if counter > 1000 {
                return Err(anyhow::anyhow!(
                    "Failed to generate unique branch name for: {}",
                    base_name
                ));
            }
        }
    }

    /// リポジトリのパスを取得
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }
}

/// ブランチ名をサニタイズ
fn sanitize_branch_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '/' {
                c
            } else if c == ' ' {
                '-'
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, WorktreeManager) {
        let temp = TempDir::new().unwrap();

        // git init
        Command::new("git")
            .current_dir(temp.path())
            .args(["init"])
            .output()
            .unwrap();

        // git config（テスト用）
        Command::new("git")
            .current_dir(temp.path())
            .args(["config", "user.email", "test@test.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp.path())
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        // 初期コミット（署名をスキップ）
        Command::new("git")
            .current_dir(temp.path())
            .args(["commit", "--allow-empty", "-m", "init", "--no-gpg-sign"])
            .output()
            .unwrap();

        let manager = WorktreeManager::new(temp.path()).unwrap();
        (temp, manager)
    }

    #[test]
    fn test_sanitize_branch_name() {
        assert_eq!(sanitize_branch_name("feature/test"), "feature/test");
        assert_eq!(sanitize_branch_name("feature test"), "feature-test");
        assert_eq!(sanitize_branch_name("feature@test"), "feature_test");
        assert_eq!(sanitize_branch_name("my-branch_name"), "my-branch_name");
    }

    #[test]
    fn test_new_from_git_repo() {
        let (temp, manager) = setup_test_repo();
        assert_eq!(manager.repo_path(), temp.path());
    }

    #[test]
    fn test_new_from_non_git_repo() {
        let temp = TempDir::new().unwrap();
        let result = WorktreeManager::new(temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_branch_exists() {
        let (_temp, manager) = setup_test_repo();

        // masterまたはmainブランチが存在するはず
        let master_exists = manager.branch_exists("master").unwrap();
        let main_exists = manager.branch_exists("main").unwrap();
        assert!(master_exists || main_exists);

        // 存在しないブランチ
        assert!(!manager.branch_exists("nonexistent-branch").unwrap());
    }

    #[test]
    fn test_generate_unique_branch() {
        let (temp, manager) = setup_test_repo();

        // 存在しないブランチはそのまま
        let branch1 = manager.generate_unique_branch("feature").unwrap();
        assert_eq!(branch1, "feature");

        // ブランチを作成
        manager.create(&branch1, temp.path()).unwrap();

        // 同じ名前を要求すると -2 が付く
        let branch2 = manager.generate_unique_branch("feature").unwrap();
        assert_eq!(branch2, "feature-2");
    }

    #[test]
    fn test_create_and_list_worktree() {
        let (temp, manager) = setup_test_repo();

        // 最初は1つ（main/masterのみ）
        let initial_list = manager.list().unwrap();
        assert_eq!(initial_list.len(), 1);
        assert!(initial_list[0].is_main);

        // Worktreeを作成
        let wt_path = manager.create("test-branch", temp.path()).unwrap();
        assert!(wt_path.exists());

        // 2つになっている
        let list = manager.list().unwrap();
        assert_eq!(list.len(), 2);

        // 作成したworktreeが含まれている
        let created_wt = list.iter().find(|wt| !wt.is_main).unwrap();
        assert_eq!(created_wt.branch, "test-branch");
    }

    #[test]
    fn test_create_and_remove_worktree() {
        let (temp, manager) = setup_test_repo();

        let wt_path = manager.create("test-branch", temp.path()).unwrap();
        assert!(wt_path.exists());

        let list = manager.list().unwrap();
        assert_eq!(list.len(), 2);

        manager.remove(&wt_path).unwrap();
        assert!(!wt_path.exists());

        let list_after = manager.list().unwrap();
        assert_eq!(list_after.len(), 1);
    }

    #[test]
    fn test_create_with_relative_base_dir() {
        let (_temp, manager) = setup_test_repo();

        // 相対パスでbase_dirを指定
        let wt_path = manager
            .create("feature/new", Path::new(".worktrees"))
            .unwrap();

        assert!(wt_path.exists());
        assert!(wt_path.to_str().unwrap().contains("feature-new"));

        // クリーンアップ
        manager.remove(&wt_path).unwrap();
    }
}
