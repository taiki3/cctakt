# cctakt (シーシー・タクト)

複数の Claude Code エージェントを Git Worktree で並列管理する TUI オーケストレーター

![cctakt screenshot](cctakt.png)

## 特徴

- **並列実行**: Git Worktree で複数タスクを同時実行
- **指揮者モード**: 左ペインで指揮者 Claude Code が全体を統括
- **ワーカー管理**: 右ペインで各ワーカーの進捗をリアルタイム監視
- **自動レビュー**: タスク完了時に diff を表示、ワンキーでマージ
- **GitHub Issues連携**: Issue から直接ワーカーを作成
- **プラン機能**: .cctakt/plan.json でタスクを定義、自動実行

## インストール

```bash
cargo install --path .
```

## 使い方

### TUI起動

```bash
cctakt
```

### キーバインド

#### グローバル
| キー | 動作 |
|------|------|
| Ctrl+Q | 終了 |
| Ctrl+T | テーマ切替 |
| Ctrl+I / F2 | Issue picker |
| Ctrl+W | アクティブエージェント終了 |
| Ctrl+N/P | タブ切替 |
| Ctrl+1-9 | タブ番号で切替 |

#### ナビゲーションモード
| キー | 動作 |
|------|------|
| h/l | 左右ペイン移動 |
| j/k | ワーカー間移動 |
| i / Enter | 入力モードへ |

#### 入力モード
| キー | 動作 |
|------|------|
| Esc | ナビゲーションモードへ |
| その他 | エージェントに送信 |

#### レビューモード（タスク完了時）
| キー | 動作 |
|------|------|
| Enter / m | マージ |
| q / c | キャンセル |
| j/k | スクロール |
| PageUp/Down | ページスクロール |

### CLIコマンド

```bash
cctakt init          # 初期化
cctakt status        # 環境状態確認
cctakt issues        # GitHub Issues一覧
cctakt run [plan]    # プランをCLIモードで実行
```

## 指揮者モード

指揮者（左ペインの Claude Code）に `.cctakt/plan.json` を書かせることで、複数ワーカーを自動起動できます。

```json
{
  "version": 1,
  "description": "タスク説明",
  "tasks": [
    {
      "id": "worker-1",
      "action": {
        "type": "create_worker",
        "branch": "feat/example",
        "task_description": "実装内容の詳細"
      },
      "status": "pending"
    }
  ]
}
```

## 設定

`.cctakt.toml` で設定:

```toml
branch_prefix = "cctakt"
worktree_dir = ".worktrees"
theme = "cyberpunk"

[github]
repository = "owner/repo"
```

## Tech Stack

- Rust
- ratatui (TUI)
- portable-pty (PTY管理)
- tokio (非同期ランタイム)
