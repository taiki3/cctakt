# cctakt (シーシー・タクト)

**Multi-agent Orchestrator for Claude Code powered by Git Worktree.**

![cctakt screenshot](cctakt.png)

cctakt は、複数の Claude Code エージェントを Git Worktree で管理し、並列でコーディングタスクを実行するための Rust 製 TUI アプリケーションです。

## 特徴

- **並列実行**: Git Worktree を活用し、複数のタスクを同時並行で実行
- **指揮者モード**: メインリポジトリで Claude Code を「指揮者」として起動し、plan.json を通じてワーカーを統括
- **ワーカー管理**: 各ワーカーの PTY 出力をリアルタイムで確認・操作可能
- **自動レビュー**: ワーカー完了時に diff を表示し、マージ判断をサポート
- **GitHub Issues 連携**: Issue からワーカーを自動生成、ブランチ名を提案
- **プラン機能**: `.cctakt/plan.json` を通じた構造化タスク管理
- **テーマ**: 6種類のカラーテーマ（Cyberpunk, Monokai, Dracula, Nord, Arctic Aurora, Minimal）

## インストール

```bash
# リポジトリをクローン
git clone https://github.com/your-username/cctakt.git
cd cctakt

# ビルド
cargo build --release

# インストール（オプション）
cargo install --path .
```

## 使い方

```bash
# TUI を起動
cctakt

# プロジェクトを初期化
cctakt init

# 環境設定を確認
cctakt status

# GitHub Issues を一覧表示
cctakt issues

# プランを実行（CLI モード）
cctakt run .cctakt/plan.json
```

## キーバインド

### グローバル

| キー | 説明 |
|------|------|
| `Ctrl+Q` | 終了 |
| `Ctrl+T` | テーマピッカーを開く |
| `Ctrl+I` / `F2` | Issue ピッカーを開く |
| `Ctrl+W` | アクティブなエージェントを閉じる |
| `Ctrl+N` | 次のタブへ |
| `Ctrl+P` | 前のタブへ |
| `Ctrl+1-9` / `Alt+1-9` | タブを番号で切り替え |

### ナビゲーションモード

| キー | 説明 |
|------|------|
| `h` | 左ペインへ移動 |
| `l` | 右ペインへ移動 |
| `j` | 次のワーカーへ（右ペイン時） |
| `k` | 前のワーカーへ（右ペイン時） |
| `i` / `Enter` | 入力モードへ切り替え |

### 入力モード

| キー | 説明 |
|------|------|
| `Esc` | ナビゲーションモードへ戻る |
| 任意のキー | エージェントへ入力を送信 |

### レビューモード

| キー | 説明 |
|------|------|
| `j` / `↓` | 下へスクロール |
| `k` / `↑` | 上へスクロール |
| `d` / `Ctrl+D` | 半ページ下へ |
| `u` / `Ctrl+U` | 半ページ上へ |
| `g` | 先頭へ |
| `G` | 末尾へ |
| `m` / `Enter` | マージを実行 |
| `Esc` / `q` | レビューをキャンセル |

## 指揮者モードと plan.json

cctakt は「指揮者モード」をサポートしています。メインリポジトリで Claude Code を起動し、`.cctakt/plan.json` にプランを書き込むことで、cctakt がワーカーを自動的に生成・管理します。

### plan.json の構造

```json
{
  "version": 1,
  "description": "タスクの説明",
  "tasks": [
    {
      "id": "worker-1",
      "action": {
        "type": "create_worker",
        "branch": "feat/example",
        "task_description": "実装内容の詳細"
      },
      "status": "pending"
    },
    {
      "id": "review-1",
      "action": {
        "type": "request_review",
        "branch": "feat/example",
        "after_task": "worker-1"
      },
      "status": "pending"
    }
  ]
}
```

### サポートされるアクション

| タイプ | 説明 |
|--------|------|
| `create_worker` | Worktree を作成し、ワーカーエージェントを起動 |
| `create_pr` | プルリクエストを作成 |
| `merge_branch` | ブランチをマージ |
| `cleanup_worktree` | Worktree を削除 |
| `run_command` | コマンドを実行 |
| `notify` | 通知メッセージを表示 |
| `request_review` | レビューモードを開始 |

### タスクステータス

- `pending`: 実行待ち
- `running`: 実行中
- `completed`: 完了
- `failed`: 失敗
- `skipped`: スキップ

## 設定ファイル

プロジェクトルートに `.cctakt.toml` を配置して設定をカスタマイズできます。

```toml
# Worktree の保存先
worktree_dir = ".worktrees"

# ブランチ名のプレフィックス
branch_prefix = "cctakt"

# カラーテーマ: cyberpunk, monokai, dracula, nord, arctic, minimal
theme = "cyberpunk"

[github]
# Issue を自動取得するか
auto_fetch_issues = false
# リポジトリ（owner/repo 形式）
repository = "owner/repo"
# フィルタするラベル
labels = ["cctakt", "good first issue"]

[anthropic]
# Anthropic API キー（環境変数 ANTHROPIC_API_KEY でも設定可能）
# api_key = "sk-ant-..."
# 使用するモデル
model = "claude-sonnet-4-20250514"
# 最大トークン数
max_tokens = 1024
# PR 説明を自動生成するか
auto_generate_pr_description = true

[keybindings]
new_agent = "ctrl+t"
close_agent = "ctrl+w"
next_tab = "tab"
prev_tab = "shift+tab"
quit = "ctrl+q"
```

## Tech Stack

| カテゴリ | 技術 |
|----------|------|
| 言語 | Rust (Edition 2024) |
| TUI | [ratatui](https://github.com/ratatui-org/ratatui) |
| ターミナル | [portable-pty](https://github.com/wez/wezterm/tree/main/pty) + [vt100](https://crates.io/crates/vt100) |
| CLI | [clap](https://github.com/clap-rs/clap) |
| HTTP | [ureq](https://github.com/algesten/ureq) |
| 設定 | [toml](https://crates.io/crates/toml) + [serde](https://serde.rs/) |

## アーキテクチャ

```
cctakt (TUI)
├── 指揮者 Claude Code (メインリポジトリ)
│   └── .cctakt/plan.json にプラン書き込み
│
└── Worker Claude Code (各 Worktree)
    └── 実際のタスク実行
```

## ライセンス

MIT
