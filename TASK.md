# Task: Phase 3 - UX Improvements

## Overview
ユーザー体験を向上させるUIコンポーネントと機能を実装する。
これらは独立したモジュールとして実装し、後でメインのTUIに統合する。

## Prerequisites
- Phase 1 (AgentManager) と Phase 2 (WorktreeManager) は並行開発中
- このPhaseでは統合を想定したインターフェースで独立モジュールを作成

---

## Task 1: 入力ダイアログモジュール (`src/dialog.rs`)

エージェント追加時などに使う汎用的な入力ダイアログ。

```rust
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub struct InputDialog {
    title: String,
    prompt: String,
    input: String,
    cursor_position: usize,
    visible: bool,
}

impl InputDialog {
    pub fn new(title: impl Into<String>, prompt: impl Into<String>) -> Self;

    /// ダイアログを表示
    pub fn show(&mut self);

    /// ダイアログを非表示
    pub fn hide(&mut self);

    /// 表示中かどうか
    pub fn is_visible(&self) -> bool;

    /// 現在の入力値を取得
    pub fn value(&self) -> &str;

    /// 入力値をクリア
    pub fn clear(&mut self);

    /// キー入力を処理（Enter で Some(value) を返す、Esc で None）
    pub fn handle_key(&mut self, key: KeyCode) -> Option<DialogResult>;

    /// 描画
    pub fn render(&self, f: &mut Frame, area: Rect);
}

pub enum DialogResult {
    Submit(String),
    Cancel,
}
```

### 描画イメージ
```
┌─ New Agent ──────────────────────────┐
│                                      │
│  Enter task description:             │
│  ┌──────────────────────────────┐    │
│  │ Fix the login bug_           │    │
│  └──────────────────────────────┘    │
│                                      │
│  [Enter] Submit  [Esc] Cancel        │
└──────────────────────────────────────┘
```

---

## Task 2: ステータスバーモジュール (`src/statusbar.rs`)

各エージェントの状態を一覧表示するステータスバー。

```rust
pub struct AgentStatusInfo {
    pub id: usize,
    pub name: String,
    pub status: AgentStatusKind,
    pub is_active: bool,
}

pub enum AgentStatusKind {
    Running,
    Idle,
    Ended,
    Error,
}

pub struct StatusBar {
    agents: Vec<AgentStatusInfo>,
}

impl StatusBar {
    pub fn new() -> Self;

    /// エージェント情報を更新
    pub fn update(&mut self, agents: Vec<AgentStatusInfo>);

    /// 描画（画面下部に1-2行で表示）
    pub fn render(&self, f: &mut Frame, area: Rect);
}
```

### 描画イメージ
```
───────────────────────────────────────────────────────────────────────
 [1] feat/auth ● Running  [2] fix/api ● Running  [3] docs ○ Ended
```

- `●` = Running (緑)
- `○` = Ended (グレー)
- `✗` = Error (赤)

---

## Task 3: マージマネージャー (`src/merge.rs`)

完了したエージェントのブランチをメインにマージする機能。

```rust
use std::path::Path;
use anyhow::Result;

pub struct MergeManager {
    repo_path: std::path::PathBuf,
}

pub struct MergePreview {
    pub branch: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub conflicts: Vec<String>,
}

impl MergeManager {
    pub fn new(repo_path: impl Into<std::path::PathBuf>) -> Self;

    /// マージ前のプレビューを取得
    pub fn preview(&self, branch: &str) -> Result<MergePreview>;

    /// 差分を取得（git diff の出力）
    pub fn diff(&self, branch: &str) -> Result<String>;

    /// マージを実行
    pub fn merge(&self, branch: &str, message: Option<&str>) -> Result<()>;

    /// マージをキャンセル（コンフリクト時）
    pub fn abort(&self) -> Result<()>;

    /// ブランチを削除
    pub fn delete_branch(&self, branch: &str) -> Result<()>;
}
```

### gitコマンドマッピング
- `preview()` → `git diff --stat main...<branch>`
- `diff()` → `git diff main...<branch>`
- `merge()` → `git merge <branch> -m "<message>"`
- `abort()` → `git merge --abort`
- `delete_branch()` → `git branch -d <branch>`

---

## Task 4: 差分ビューア (`src/diffview.rs`)

マージ前に差分を確認するためのビューア。

```rust
pub struct DiffView {
    diff_content: String,
    scroll: u16,
    syntax_highlight: bool,
}

impl DiffView {
    pub fn new(diff: String) -> Self;

    /// スクロール操作
    pub fn scroll_up(&mut self, lines: u16);
    pub fn scroll_down(&mut self, lines: u16);

    /// 描画
    pub fn render(&self, f: &mut Frame, area: Rect);
}
```

### 描画イメージ
```
┌─ Diff: feat/auth -> main ────────────────────────────────────────────┐
│ src/auth.rs                                                          │
│ @@ -10,6 +10,15 @@                                                    │
│   fn login(user: &str, pass: &str) -> Result<Token> {                │
│ +     // Validate input                                               │
│ +     if user.is_empty() || pass.is_empty() {                        │
│ +         return Err(AuthError::InvalidInput);                       │
│ +     }                                                               │
│       let hash = hash_password(pass);                                 │
│                                                                       │
│ [↑/↓] Scroll  [Enter] Merge  [Esc] Cancel                            │
└──────────────────────────────────────────────────────────────────────┘
```

- `+` 行 = 緑
- `-` 行 = 赤
- `@@` 行 = シアン

---

## Implementation Notes

1. **独立性**: 各モジュールは単体でコンパイル・テスト可能にする
2. **インターフェース**: Phase 1/2 との統合ポイントを明確に
3. **依存関係**: ratatui, crossterm, anyhow のみ使用

## Testing

```bash
# ビルド確認
cargo build

# 各モジュールの単体テスト
cargo test dialog
cargo test statusbar
cargo test merge
cargo test diffview
```

## Files to Create

- `src/dialog.rs`
- `src/statusbar.rs`
- `src/merge.rs`
- `src/diffview.rs`
- `src/lib.rs` に各モジュールを追加

## Integration Points

```rust
// 統合時の使用例
// 1. エージェント追加時
let mut dialog = InputDialog::new("New Agent", "Enter task:");
dialog.show();
// ... handle_key でユーザー入力を取得
let task = dialog.value();

// 2. ステータスバー更新
statusbar.update(agent_manager.list().iter().map(|a| AgentStatusInfo {
    id: a.id,
    name: a.name.clone(),
    status: a.status.into(),
    is_active: a.id == agent_manager.active_index(),
}).collect());

// 3. マージフロー
let preview = merge_manager.preview("feat/auth")?;
if preview.conflicts.is_empty() {
    let diff = merge_manager.diff("feat/auth")?;
    // DiffView で表示、ユーザーが確認後
    merge_manager.merge("feat/auth", Some("Merge feat/auth"))?;
}
```
