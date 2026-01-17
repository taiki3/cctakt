# cctakt Development Guidelines

## 重要なルール

**Rustのエディションは絶対に2024から下げてはいけない**

## プロジェクト概要

cctakt は複数の Claude Code エージェントを Git Worktree で管理する TUI オーケストレーターです。

## アーキテクチャ

```
cctakt (TUI)
├── 指揮者 Claude Code (メインリポジトリ)
│   └── .cctakt/plan.json にプラン書き込み
│
└── Worker Claude Code (各 Worktree)
    └── 実際のタスク実行
```

## 指揮者モード

指揮者として動作する場合は `.claude/orchestrator.md` を参照してください。

プラン作成例:
```bash
mkdir -p .cctakt && cat > .cctakt/plan.json << 'EOF'
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
EOF
```

## モジュール構成

- `src/plan.rs` - プラン管理（指揮者↔cctakt通信）
- `src/worktree.rs` - Git Worktree管理
- `src/agent.rs` - PTYエージェント管理
- `src/github.rs` - GitHub API
- `src/anthropic.rs` - Anthropic API（PR本文生成用）