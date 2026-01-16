# Task: Phase 4 - Advanced Features

## Overview
発展的な機能を実装する。GitHub連携、設定ファイル対応など。
これらは独立したモジュールとして実装し、後でメインに統合する。

## Prerequisites
- Phase 1-3 は並行開発中
- このPhaseでは統合を想定したインターフェースで独立モジュールを作成

---

## Task 1: 設定ファイル対応 (`src/config.rs`)

`.cctakt.toml` による設定ファイル対応。

```rust
use std::path::{Path, PathBuf};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// ワークツリーを作成するベースディレクトリ
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: PathBuf,

    /// デフォルトのブランチプレフィックス
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,

    /// GitHub設定
    #[serde(default)]
    pub github: GitHubConfig,

    /// キーバインド設定
    #[serde(default)]
    pub keybindings: KeyBindings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitHubConfig {
    /// 自動的にIssueからタスクを取得するか
    pub auto_fetch_issues: bool,

    /// 対象リポジトリ（owner/repo形式）
    pub repository: Option<String>,

    /// 取得するラベル（例: "cctakt", "good first issue"）
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    pub new_agent: String,      // default: "ctrl+t"
    pub close_agent: String,    // default: "ctrl+w"
    pub next_tab: String,       // default: "tab"
    pub prev_tab: String,       // default: "shift+tab"
    pub quit: String,           // default: "ctrl+q"
}

fn default_worktree_dir() -> PathBuf {
    PathBuf::from(".worktrees")
}

fn default_branch_prefix() -> String {
    "cctakt".to_string()
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            new_agent: "ctrl+t".to_string(),
            close_agent: "ctrl+w".to_string(),
            next_tab: "tab".to_string(),
            prev_tab: "shift+tab".to_string(),
            quit: "ctrl+q".to_string(),
        }
    }
}

impl Config {
    /// 設定ファイルを読み込む（見つからなければデフォルト）
    pub fn load() -> Result<Self>;

    /// 指定パスから読み込む
    pub fn load_from(path: &Path) -> Result<Self>;

    /// 設定ファイルを保存
    pub fn save(&self) -> Result<()>;

    /// デフォルト設定ファイルを生成
    pub fn generate_default(path: &Path) -> Result<()>;
}
```

### 設定ファイル例 (`.cctakt.toml`)
```toml
worktree_dir = ".worktrees"
branch_prefix = "cctakt"

[github]
auto_fetch_issues = true
repository = "user/repo"
labels = ["cctakt", "good first issue"]

[keybindings]
new_agent = "ctrl+t"
close_agent = "ctrl+w"
next_tab = "tab"
prev_tab = "shift+tab"
quit = "ctrl+q"
```

---

## Task 2: GitHub Issues連携 (`src/github.rs`)

GitHub APIを使ってIssueを取得し、エージェントのタスクとして使う。

```rust
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub labels: Vec<Label>,
    pub state: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Label {
    pub name: String,
    pub color: String,
}

pub struct GitHubClient {
    repository: String,  // "owner/repo"
    token: Option<String>,
}

impl GitHubClient {
    /// 環境変数 GITHUB_TOKEN または gh auth token から認証
    pub fn new(repository: &str) -> Result<Self>;

    /// Issueを取得（フィルタ付き）
    pub fn fetch_issues(&self, labels: &[&str], state: &str) -> Result<Vec<Issue>>;

    /// 単一のIssueを取得
    pub fn get_issue(&self, number: u64) -> Result<Issue>;

    /// Issueにコメントを追加
    pub fn add_comment(&self, number: u64, body: &str) -> Result<()>;

    /// Issueをクローズ
    pub fn close_issue(&self, number: u64) -> Result<()>;
}
```

### 実装ノート

**認証トークンの取得優先順位:**
1. 環境変数 `GITHUB_TOKEN`
2. `gh auth token` コマンドの出力（GitHub CLI）

**APIエンドポイント:**
- Issues一覧: `GET /repos/{owner}/{repo}/issues`
- Issue詳細: `GET /repos/{owner}/{repo}/issues/{issue_number}`
- コメント追加: `POST /repos/{owner}/{repo}/issues/{issue_number}/comments`

**HTTPクライアント:**
- `ureq` クレートを使用（軽量、同期的）
- プロキシ対応が必要な場合は環境変数 `HTTP_PROXY` を考慮

```rust
// 実装例
pub fn fetch_issues(&self, labels: &[&str], state: &str) -> Result<Vec<Issue>> {
    let labels_param = labels.join(",");
    let url = format!(
        "https://api.github.com/repos/{}/issues?labels={}&state={}",
        self.repository, labels_param, state
    );

    let mut request = ureq::get(&url)
        .set("Accept", "application/vnd.github.v3+json")
        .set("User-Agent", "cctakt");

    if let Some(ref token) = self.token {
        request = request.set("Authorization", &format!("Bearer {}", token));
    }

    let response = request.call()?;
    let issues: Vec<Issue> = response.into_json()?;
    Ok(issues)
}
```

---

## Task 3: Issue選択UI (`src/issue_picker.rs`)

Issueを一覧から選択するUI。

```rust
pub struct IssuePicker {
    issues: Vec<Issue>,
    selected_index: usize,
    scroll_offset: usize,
    loading: bool,
    error: Option<String>,
}

impl IssuePicker {
    pub fn new() -> Self;

    /// Issueリストを設定
    pub fn set_issues(&mut self, issues: Vec<Issue>);

    /// ローディング状態を設定
    pub fn set_loading(&mut self, loading: bool);

    /// エラーを設定
    pub fn set_error(&mut self, error: Option<String>);

    /// キー入力を処理
    pub fn handle_key(&mut self, key: KeyCode) -> Option<IssuePickerResult>;

    /// 描画
    pub fn render(&self, f: &mut Frame, area: Rect);
}

pub enum IssuePickerResult {
    Selected(Issue),
    Cancel,
    Refresh,
}
```

### 描画イメージ
```
┌─ Select Issue ───────────────────────────────────────────────────────┐
│                                                                       │
│  #123  [bug] Fix login validation error                              │
│ >#456  [feature] Add dark mode support                      ← 選択中 │
│  #789  [docs] Update README                                          │
│  #101  [refactor] Clean up authentication module                     │
│                                                                       │
│  [↑/↓] Navigate  [Enter] Select  [r] Refresh  [Esc] Cancel           │
└───────────────────────────────────────────────────────────────────────┘
```

---

## Task 4: タスクテンプレート (`src/template.rs`)

Issueからエージェントへの指示を生成するテンプレート。

```rust
pub struct TaskTemplate {
    template: String,
}

impl TaskTemplate {
    pub fn new(template: &str) -> Self;

    /// デフォルトテンプレート
    pub fn default() -> Self {
        Self::new(r#"
Please work on the following GitHub issue:

## Issue #{{number}}: {{title}}

{{body}}

## Instructions
1. Read the issue carefully
2. Implement the required changes
3. Write tests if applicable
4. Commit with message referencing the issue (e.g., "Fix #{{number}}: ...")
"#)
    }

    /// テンプレートを適用してタスク文字列を生成
    pub fn render(&self, issue: &Issue) -> String;
}
```

---

## Dependencies

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
toml = "0.8"
ureq = { version = "2", features = ["json"] }
```

---

## Testing

```bash
# ビルド確認
cargo build

# 単体テスト
cargo test config
cargo test github
cargo test issue_picker
cargo test template

# 設定ファイル生成テスト
cargo run -- --generate-config
```

### GitHub APIテスト（手動）
```bash
# gh CLI がインストールされている前提
export GITHUB_TOKEN=$(gh auth token)

# テストコード実行
cargo test github_integration -- --ignored
```

---

## Files to Create

- `src/config.rs`
- `src/github.rs`
- `src/issue_picker.rs`
- `src/template.rs`
- `src/lib.rs` に各モジュールを追加

---

## Integration Points

```rust
// 統合時の使用例

// 1. 設定読み込み
let config = Config::load()?;

// 2. GitHub Issue取得
let client = GitHubClient::new(&config.github.repository.unwrap())?;
let issues = client.fetch_issues(&config.github.labels, "open")?;

// 3. Issue選択UI
let mut picker = IssuePicker::new();
picker.set_issues(issues);
// ... render and handle_key

// 4. 選択されたIssueからタスク生成
if let IssuePickerResult::Selected(issue) = result {
    let template = TaskTemplate::default();
    let task = template.render(&issue);

    // エージェント起動時にタスクを渡す
    let branch = format!("{}/issue-{}", config.branch_prefix, issue.number);
    // worktree_manager.create(&branch, &config.worktree_dir)?;
    // agent_manager.add(&issue.title, worktree_path)?;
    // agent.send_bytes(task.as_bytes());
}
```

---

## Notes

1. **プロキシ対応**: 環境変数 `HTTP_PROXY`, `HTTPS_PROXY` を考慮
2. **レート制限**: GitHub APIのレート制限に注意（認証なしで60回/時間）
3. **オフライン対応**: API接続失敗時はエラー表示して続行可能に
