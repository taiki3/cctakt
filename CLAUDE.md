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

## Claude Code リファレンスドキュメント

`ref/` ディレクトリに公式ドキュメントを配置。

### ref/cli.md - CLIリファレンス

**主な用途**: 非対話モード・構造化出力

- `-p "prompt"`: 非対話モードで即座に実行して終了
- `--output-format stream-json`: JSONストリームで進捗を取得
- `--max-turns N`: ターン数制限（無限ループ防止）
- `--json-schema`: 出力を特定のスキーマに強制

**ワークフロー改善への応用**:
PTYでscreen scrapingする代わりに、`claude -p "task" --output-format stream-json` でJSONパースすれば完了検知が確実になる。

### ref/hooks.md - Hooksリファレンス

**主な用途**: Claude Codeイベントへのフック

重要なイベント:
- `Stop`: エージェントが作業完了時に発火。`"decision": "block"` で継続を強制可能
- `SubagentStop`: サブエージェント完了時
- `PostToolUse`: ツール使用後（例: `Write|Edit` でコミット検知）
- `SessionEnd`: セッション終了時

**プロンプトベースhooks** (`type: "prompt"`):
LLMで完了判定が可能。`Stop` hookで「タスク完了したか？」をLLMに評価させられる。

**ワークフロー改善への応用**:
`.claude/settings.json` に `Stop` hookを設定し、cctaktにシグナルを送れば、screen scrapingより信頼性の高い完了検知が実現できる。

### ref/plugin.md - Pluginリファレンス

**主な用途**: 再利用可能な拡張パッケージ

- Hooksをプラグインとしてパッケージ化
- `hooks/hooks.json` でプラグイン固有のhooksを定義
- `${CLAUDE_PLUGIN_ROOT}` で相対パス参照

**ワークフロー改善への応用**:
cctakt用の完了通知hookをプラグイン化して、各worktreeで自動有効化できる。

### ref/slash.md - Slashコマンドリファレンス

**主な用途**: カスタムコマンド定義

- `.claude/commands/*.md` でプロジェクト固有コマンド
- frontmatterで `allowed-tools`, `hooks` 定義可能
- `$ARGUMENTS` でパラメータ受け取り

**cctaktでの応用**: 特になし（対話的操作向け）

### ref/checkpointing.md - チェックポイント機能

**主な用途**: 編集の自動追跡とリワインド

- `/rewind` でコード/会話を過去に戻せる
- Bashコマンドによる変更は追跡されない

**cctaktでの応用**: レビュー時に問題があれば `/rewind` で戻せる

### ref/interactive.md - 対話モードリファレンス

**主な用途**: キーボードショートカット・Vimモード

- `Ctrl+B`: コマンドをバックグラウンド化
- `!command`: 直接Bash実行

**cctaktでの応用**: 特になし（人間の対話操作向け）

---

## 自動ワークフロー推奨アプローチ

**結論: 非対話モード (`-p --output-format stream-json`) が最適**

理由:
1. 設定ファイル不要（hooks/plugin配置不要）
2. プロセス終了 = 完了（曖昧さゼロ）
3. `--max-turns N` で無限ループ防止
4. JSONストリームで進捗取得可能