# Hooksリファレンス

> このページでは、Claude Codeでhooksを実装するためのリファレンスドキュメントを提供します。

<Tip>
  クイックスタートガイドと例については、[Claude Code hooksを始める](/ja/hooks-guide)を参照してください。
</Tip>

## 設定

Claude Code hooksは[設定ファイル](/ja/settings)で設定されます：

* `~/.claude/settings.json` - ユーザー設定
* `.claude/settings.json` - プロジェクト設定
* `.claude/settings.local.json` - ローカルプロジェクト設定（コミットされない）
* 管理ポリシー設定

<Note>
  エンタープライズ管理者は`allowManagedHooksOnly`を使用して、ユーザー、プロジェクト、およびプラグインhooksをブロックできます。[Hook設定](/ja/settings#hook-configuration)を参照してください。
</Note>

### 構造

Hooksはマッチャーで整理され、各マッチャーは複数のhooksを持つことができます：

```json  theme={null}
{
  "hooks": {
    "EventName": [
      {
        "matcher": "ToolPattern",
        "hooks": [
          {
            "type": "command",
            "command": "your-command-here"
          }
        ]
      }
    ]
  }
}
```

* **matcher**: ツール名にマッチするパターン、大文字小文字を区別します（`PreToolUse`、`PermissionRequest`、`PostToolUse`にのみ適用）
  * 単純な文字列は正確にマッチします：`Write`はWriteツールのみにマッチします
  * 正規表現をサポートします：`Edit|Write`または`Notebook.*`
  * `*`を使用してすべてのツールにマッチします。空の文字列（`""`）を使用することもできます、または`matcher`を空白のままにします。
* **hooks**: パターンがマッチしたときに実行するhooksの配列
  * `type`: Hook実行タイプ - bashコマンドの場合は`"command"`、LLMベースの評価の場合は`"prompt"`
  * `command`: （`type: "command"`の場合）実行するbashコマンド（`$CLAUDE_PROJECT_DIR`環境変数を使用できます）
  * `prompt`: （`type: "prompt"`の場合）評価のためにLLMに送信するプロンプト
  * `timeout`: （オプション）hookが実行される時間（秒単位）、その特定のhookをキャンセルする前に

`UserPromptSubmit`、`Stop`、`SubagentStop`などのマッチャーを使用しないイベントの場合、マッチャーフィールドを省略できます：

```json  theme={null}
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/prompt-validator.py"
          }
        ]
      }
    ]
  }
}
```

### プロジェクト固有のHookスクリプト

環境変数`CLAUDE_PROJECT_DIR`（Claude Codeがhookコマンドをスポーンするときのみ利用可能）を使用して、プロジェクトに保存されたスクリプトを参照し、Claudeの現在のディレクトリに関係なく動作することを保証できます：

```json  theme={null}
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/check-style.sh"
          }
        ]
      }
    ]
  }
}
```

### プラグインhooks

[プラグイン](/ja/plugins)はユーザーおよびプロジェクトhooksとシームレスに統合するhooksを提供できます。プラグインhooksはプラグインが有効になると、自動的に設定とマージされます。

**プラグインhooksの動作方法**：

* プラグインhooksはプラグインの`hooks/hooks.json`ファイルまたは`hooks`フィールドへのカスタムパスで指定されたファイルで定義されます。
* プラグインが有効になると、そのhooksはユーザーおよびプロジェクトhooksとマージされます
* 異なるソースからの複数のhooksが同じイベントに応答できます
* プラグインhooksは`${CLAUDE_PLUGIN_ROOT}`環境変数を使用してプラグインファイルを参照します

**プラグインhook設定の例**：

```json  theme={null}
{
  "description": "Automatic code formatting",
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/scripts/format.sh",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

<Note>
  プラグインhooksは通常のhooksと同じ形式を使用し、hookの目的を説明するためのオプションの`description`フィールドがあります。
</Note>

<Note>
  プラグインhooksはカスタムhooksと一緒に実行されます。複数のhooksがイベントにマッチする場合、すべてが並列で実行されます。
</Note>

**プラグイン用の環境変数**：

* `${CLAUDE_PLUGIN_ROOT}`: プラグインディレクトリへの絶対パス
* `${CLAUDE_PROJECT_DIR}`: プロジェクトルートディレクトリ（プロジェクトhooksと同じ）
* すべての標準環境変数が利用可能です

プラグインhooksの作成の詳細については、[プラグインコンポーネントリファレンス](/ja/plugins-reference#hooks)を参照してください。

### Skills、Agents、およびSlash Commandsのhooks

設定ファイルとプラグインに加えて、hooksは[Skills](/ja/skills)、[subagents](/ja/sub-agents)、および[slash commands](/ja/slash-commands)でフロントマターを使用して直接定義できます。これらのhooksはコンポーネントのライフサイクルにスコープされ、そのコンポーネントがアクティブな場合にのみ実行されます。

**サポートされるイベント**: `PreToolUse`、`PostToolUse`、および`Stop`

**Skillの例**：

```yaml  theme={null}
---
name: secure-operations
description: Perform operations with security checks
hooks:
  PreToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "./scripts/security-check.sh"
---
```

**agentの例**：

```yaml  theme={null}
---
name: code-reviewer
description: Review code changes
hooks:
  PostToolUse:
    - matcher: "Edit|Write"
      hooks:
        - type: command
          command: "./scripts/run-linter.sh"
---
```

コンポーネントスコープのhooksは設定ベースのhooksと同じ設定形式に従いますが、コンポーネントの実行が完了すると自動的にクリーンアップされます。

**skillsおよびslash commands用の追加オプション**：

* `once`: `true`に設定して、hookをセッションごとに1回だけ実行します。最初の成功した実行後、hookは削除されます。注：このオプションは現在、skillsおよびslash commandsでのみサポートされており、agentsではサポートされていません。

## プロンプトベースのhooks

bashコマンドhooks（`type: "command"`）に加えて、Claude Codeはプロンプトベースのhooksをサポートしています（`type: "prompt"`）。これはLLMを使用してアクションを許可またはブロックするかを評価します。プロンプトベースのhooksは現在、`Stop`および`SubagentStop` hooksでのみサポートされており、インテリジェントなコンテキスト認識の決定を可能にします。

### プロンプトベースのhooksの動作方法

bashコマンドを実行する代わりに、プロンプトベースのhooksは：

1. hookの入力とプロンプトを高速LLM（Haiku）に送信します
2. LLMは決定を含む構造化JSONで応答します
3. Claude Codeは決定を自動的に処理します

### 設定

```json  theme={null}
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "prompt",
            "prompt": "Evaluate if Claude should stop: $ARGUMENTS. Check if all tasks are complete."
          }
        ]
      }
    ]
  }
}
```

**フィールド**：

* `type`: `"prompt"`である必要があります
* `prompt`: LLMに送信するプロンプトテキスト
  * hookの入力JSONのプレースホルダーとして`$ARGUMENTS`を使用します
  * `$ARGUMENTS`が存在しない場合、入力JSONはプロンプトに追加されます
* `timeout`: （オプション）タイムアウト（秒単位）（デフォルト：30秒）

### レスポンススキーマ

LLMは以下を含むJSONで応答する必要があります：

```json  theme={null}
{
  "ok": true | false,
  "reason": "Explanation for the decision"
}
```

**レスポンスフィールド**：

* `ok`: `true`はアクションを許可し、`false`はそれを防ぎます
* `reason`: `ok`が`false`の場合は必須です。Claudeに表示される説明

### サポートされるhookイベント

プロンプトベースのhooksはすべてのhookイベントで機能しますが、以下の場合に最も有用です：

* **Stop**: Claudeが作業を続けるべきかをインテリジェントに決定します
* **SubagentStop**: subagentがそのタスクを完了したかを評価します
* **UserPromptSubmit**: LLMアシスタンスでユーザープロンプトを検証します
* **PreToolUse**: コンテキスト認識の許可決定を行います
* **PermissionRequest**: インテリジェントに許可ダイアログを許可または拒否します

### 例：インテリジェントなStop hook

```json  theme={null}
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "prompt",
            "prompt": "You are evaluating whether Claude should stop working. Context: $ARGUMENTS\n\nAnalyze the conversation and determine if:\n1. All user-requested tasks are complete\n2. Any errors need to be addressed\n3. Follow-up work is needed\n\nRespond with JSON: {\"ok\": true} to allow stopping, or {\"ok\": false, \"reason\": \"your explanation\"} to continue working.",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

### 例：カスタムロジックを使用したSubagentStop

```json  theme={null}
{
  "hooks": {
    "SubagentStop": [
      {
        "hooks": [
          {
            "type": "prompt",
            "prompt": "Evaluate if this subagent should stop. Input: $ARGUMENTS\n\nCheck if:\n- The subagent completed its assigned task\n- Any errors occurred that need fixing\n- Additional context gathering is needed\n\nReturn: {\"ok\": true} to allow stopping, or {\"ok\": false, \"reason\": \"explanation\"} to continue."
          }
        ]
      }
    ]
  }
}
```

### bashコマンドhooksとの比較

| 機能             | Bashコマンドhooks | プロンプトベースのhooks |
| -------------- | ------------- | -------------- |
| **実行**         | bashスクリプトを実行  | LLMをクエリ        |
| **決定ロジック**     | コードで実装        | LLMがコンテキストを評価  |
| **セットアップの複雑さ** | スクリプトファイルが必要  | プロンプトを設定       |
| **コンテキスト認識**   | スクリプトロジックに限定  | 自然言語理解         |
| **パフォーマンス**    | 高速（ローカル実行）    | 低速（APIコール）     |
| **ユースケース**     | 決定論的ルール       | コンテキスト認識の決定    |

### ベストプラクティス

* **プロンプトで具体的に**: LLMに評価してほしいことを明確に述べます
* **決定基準を含める**: LLMが考慮すべき要因をリストします
* **プロンプトをテストする**: LLMがユースケースに対して正しい決定を下すことを確認します
* **適切なタイムアウトを設定**: デフォルトは30秒で、必要に応じて調整します
* **複雑な決定に使用**: bashコマンドhooksは単純で決定論的なルールに適しています

プラグインhooksの作成の詳細については、[プラグインコンポーネントリファレンス](/ja/plugins-reference#hooks)を参照してください。

## Hook イベント

### PreToolUse

Claudeがツールパラメータを作成した後、ツール呼び出しを処理する前に実行されます。

**一般的なマッチャー**：

* `Task` - Subagentタスク（[subagentsドキュメント](/ja/sub-agents)を参照）
* `Bash` - シェルコマンド
* `Glob` - ファイルパターンマッチング
* `Grep` - コンテンツ検索
* `Read` - ファイル読み取り
* `Edit` - ファイル編集
* `Write` - ファイル書き込み
* `WebFetch`、`WebSearch` - Web操作

[PreToolUse決定制御](#pretooluse-decision-control)を使用して、ツールの使用を許可、拒否、または許可を求めます。

### PermissionRequest

ユーザーに許可ダイアログが表示されたときに実行されます。
[PermissionRequest決定制御](#permissionrequest-decision-control)を使用して、ユーザーに代わって許可または拒否します。

PreToolUseと同じマッチャー値を認識します。

### PostToolUse

ツールが正常に完了した直後に実行されます。

PreToolUseと同じマッチャー値を認識します。

### Notification

Claude Codeが通知を送信するときに実行されます。通知タイプでフィルタリングするマッチャーをサポートします。

**一般的なマッチャー**：

* `permission_prompt` - Claude Codeからの許可リクエスト
* `idle_prompt` - Claudeがユーザー入力を待機しているとき（60秒以上のアイドル時間後）
* `auth_success` - 認証成功通知
* `elicitation_dialog` - Claude CodeがMCPツール抽出のための入力を必要とするとき

マッチャーを使用して異なる通知タイプに対して異なるhooksを実行することも、すべての通知に対してhooksを実行するためにマッチャーを省略することもできます。

**例：異なるタイプの異なる通知**

```json  theme={null}
{
  "hooks": {
    "Notification": [
      {
        "matcher": "permission_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/permission-alert.sh"
          }
        ]
      },
      {
        "matcher": "idle_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/idle-notification.sh"
          }
        ]
      }
    ]
  }
}
```

### UserPromptSubmit

ユーザーがプロンプトを送信するときに実行され、Claudeがそれを処理する前に実行されます。これにより、プロンプト/会話に基づいて追加のコンテキストを追加したり、プロンプトを検証したり、特定のタイプのプロンプトをブロックしたりできます。

### Stop

メインのClaude Code agentが応答を終了したときに実行されます。ユーザー割り込みが原因で停止が発生した場合は実行されません。

### SubagentStop

Claude Code subagent（Taskツール呼び出し）が応答を終了したときに実行されます。

### PreCompact

Claude Codeがコンパクト操作を実行しようとする前に実行されます。

**マッチャー**：

* `manual` - `/compact`から呼び出された
* `auto` - 自動コンパクトから呼び出された（コンテキストウィンドウが満杯のため）

### SessionStart

Claude Codeが新しいセッションを開始するか、既存のセッションを再開するときに実行されます（現在、内部的には新しいセッションを開始します）。既存の問題や最近のコードベースの変更などの開発コンテキストを読み込んだり、依存関係をインストールしたり、環境変数をセットアップしたりするのに便利です。

**マッチャー**：

* `startup` - スタートアップから呼び出された
* `resume` - `--resume`、`--continue`、または`/resume`から呼び出された
* `clear` - `/clear`から呼び出された
* `compact` - 自動または手動コンパクトから呼び出された。

#### 環境変数の永続化

SessionStart hooksは`CLAUDE_ENV_FILE`環境変数にアクセスでき、後続のbashコマンドの環境変数を永続化できるファイルパスを提供します。

**例：個別の環境変数を設定する**

```bash  theme={null}
#!/bin/bash

if [ -n "$CLAUDE_ENV_FILE" ]; then
  echo 'export NODE_ENV=production' >> "$CLAUDE_ENV_FILE"
  echo 'export API_KEY=your-api-key' >> "$CLAUDE_ENV_FILE"
  echo 'export PATH="$PATH:./node_modules/.bin"' >> "$CLAUDE_ENV_FILE"
fi

exit 0
```

**例：hookからのすべての環境変更を永続化する**

セットアップが環境を変更する場合（例えば、`nvm use`）、環境をdiffして、すべての変更をキャプチャして永続化します：

```bash  theme={null}
#!/bin/bash

ENV_BEFORE=$(export -p | sort)

# Run your setup commands that modify the environment
source ~/.nvm/nvm.sh
nvm use 20

if [ -n "$CLAUDE_ENV_FILE" ]; then
  ENV_AFTER=$(export -p | sort)
  comm -13 <(echo "$ENV_BEFORE") <(echo "$ENV_AFTER") >> "$CLAUDE_ENV_FILE"
fi

exit 0
```

このファイルに書き込まれたすべての変数は、セッション中にClaude Codeが実行するすべての後続のbashコマンドで利用可能になります。

<Note>
  `CLAUDE_ENV_FILE`はSessionStart hooksでのみ利用可能です。他のhookタイプはこの変数にアクセスできません。
</Note>

### SessionEnd

Claude Codeセッションが終了するときに実行されます。クリーンアップタスク、セッション統計のログ、またはセッション状態の保存に便利です。

hookの入力の`reason`フィールドは以下のいずれかになります：

* `clear` - /clearコマンドでセッションがクリアされた
* `logout` - ユーザーがログアウトした
* `prompt_input_exit` - プロンプト入力が表示されている間にユーザーが終了した
* `other` - その他の終了理由

## Hook入力

Hooksはstdinを介してセッション情報とイベント固有のデータを含むJSONデータを受け取ります：

```typescript  theme={null}
{
  // 共通フィールド
  session_id: string
  transcript_path: string  // 会話JSONへのパス
  cwd: string              // hookが呼び出されたときの現在の作業ディレクトリ
  permission_mode: string  // 現在の許可モード："default"、"plan"、"acceptEdits"、"dontAsk"、または"bypassPermissions"

  // イベント固有フィールド
  hook_event_name: string
  ...
}
```

### PreToolUse入力

`tool_input`の正確なスキーマはツールに依存します。

```json  theme={null}
{
  "session_id": "abc123",
  "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "PreToolUse",
  "tool_name": "Write",
  "tool_input": {
    "file_path": "/path/to/file.txt",
    "content": "file content"
  },
  "tool_use_id": "toolu_01ABC123..."
}
```

### PostToolUse入力

`tool_input`と`tool_response`の正確なスキーマはツールに依存します。

```json  theme={null}
{
  "session_id": "abc123",
  "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "PostToolUse",
  "tool_name": "Write",
  "tool_input": {
    "file_path": "/path/to/file.txt",
    "content": "file content"
  },
  "tool_response": {
    "filePath": "/path/to/file.txt",
    "success": true
  },
  "tool_use_id": "toolu_01ABC123..."
}
```

### Notification入力

```json  theme={null}
{
  "session_id": "abc123",
  "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "Notification",
  "message": "Claude needs your permission to use Bash",
  "notification_type": "permission_prompt"
}
```

### UserPromptSubmit入力

```json  theme={null}
{
  "session_id": "abc123",
  "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "UserPromptSubmit",
  "prompt": "Write a function to calculate the factorial of a number"
}
```

### StopおよびSubagentStop入力

`stop_hook_active`は、Claude Codeがすでにstop hookの結果として続行している場合、trueです。この値をチェックするか、トランスクリプトを処理して、Claude Codeが無限に実行されるのを防ぎます。

```json  theme={null}
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "permission_mode": "default",
  "hook_event_name": "Stop",
  "stop_hook_active": true
}
```

### PreCompact入力

`manual`の場合、`custom_instructions`はユーザーが`/compact`に渡すものから来ます。`auto`の場合、`custom_instructions`は空です。

```json  theme={null}
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "permission_mode": "default",
  "hook_event_name": "PreCompact",
  "trigger": "manual",
  "custom_instructions": ""
}
```

### SessionStart入力

```json  theme={null}
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "permission_mode": "default",
  "hook_event_name": "SessionStart",
  "source": "startup"
}
```

### SessionEnd入力

```json  theme={null}
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "SessionEnd",
  "reason": "exit"
}
```

## Hook出力

Hooksがoutputを返してClaude Codeに戻す方法は2つあり、相互に排他的です。出力は、ブロックするかどうかと、Claudeとユーザーに表示されるべきフィードバックを通信します。

### シンプル：終了コード

Hooksは終了コード、stdout、およびstderrを通じてステータスを通信します：

* **終了コード0**: 成功。`stdout`は詳細モード（ctrl+o）でユーザーに表示されます。ただし、`UserPromptSubmit`および`SessionStart`の場合、stdoutはコンテキストに追加されます。`stdout`のJSON出力は構造化制御のために解析されます（[高度な：JSON出力](#advanced-json-output)を参照）。
* **終了コード2**: ブロッキングエラー。`stderr`のみがエラーメッセージとして使用され、Claudeにフィードバックされます。形式は`[command]: {stderr}`です。終了コード2の場合、`stdout`のJSONは処理されません。以下のhookイベントごとの動作を参照してください。
* **その他の終了コード**: ブロッキングなしのエラー。`stderr`は詳細モード（ctrl+o）でユーザーに表示され、形式は`Failed with non-blocking status code: {stderr}`です。`stderr`が空の場合、`No stderr output`が表示されます。実行は続行されます。

<Warning>
  リマインダー：終了コードが0の場合、Claude Codeはstdoutを見ません。ただし、`UserPromptSubmit` hookの場合は例外で、stdoutはコンテキストに注入されます。
</Warning>

#### 終了コード2の動作

| Hook イベント           | 動作                                     |
| ------------------- | -------------------------------------- |
| `PreToolUse`        | ツール呼び出しをブロック、stderrをClaudeに表示          |
| `PermissionRequest` | 許可を拒否、stderrをClaudeに表示                 |
| `PostToolUse`       | stderrをClaudeに表示（ツールはすでに実行）            |
| `Notification`      | N/A、stderrをユーザーのみに表示                   |
| `UserPromptSubmit`  | プロンプト処理をブロック、プロンプトを消去、stderrをユーザーのみに表示 |
| `Stop`              | 停止をブロック、stderrをClaudeに表示               |
| `SubagentStop`      | 停止をブロック、stderrをClaudeサブエージェントに表示       |
| `PreCompact`        | N/A、stderrをユーザーのみに表示                   |
| `SessionStart`      | N/A、stderrをユーザーのみに表示                   |
| `SessionEnd`        | N/A、stderrをユーザーのみに表示                   |

### 高度な：JSON出力

Hooksはより高度な制御のために`stdout`で構造化JSONを返すことができます。

<Warning>
  JSON出力はhookが終了コード0で終了した場合にのみ処理されます。hookが終了コード2（ブロッキングエラー）で終了する場合、`stderr`テキストが直接使用されます。`stdout`のJSONは無視されます。その他の非ゼロ終了コードの場合、`stderr`のみが詳細モード（ctrl+o）でユーザーに表示されます。
</Warning>

#### 共通JSONフィールド

すべてのhookタイプは、これらのオプションフィールドを含めることができます：

```json  theme={null}
{
  "continue": true, // hookの実行後にClaudeが続行するかどうか（デフォルト：true）
  "stopReason": "string", // continueがfalseの場合に表示されるメッセージ

  "suppressOutput": true, // トランスクリプトモードからstdoutを非表示（デフォルト：false）
  "systemMessage": "string" // ユーザーに表示されるオプションの警告メッセージ
}
```

`continue`がfalseの場合、hooksの実行後にClaudeは処理を停止します。

* `PreToolUse`の場合、これは`"permissionDecision": "deny"`とは異なります。これは特定のツール呼び出しのみをブロックし、Claudeに自動フィードバックを提供します。
* `PostToolUse`の場合、これは`"decision": "block"`とは異なります。これは自動フィードバックをClaudeに提供します。
* `UserPromptSubmit`の場合、これはプロンプトが処理されるのを防ぎます。
* `Stop`および`SubagentStop`の場合、これは任意の`"decision": "block"`出力よりも優先されます。
* すべての場合において、`"continue" = false`は任意の`"decision": "block"`出力よりも優先されます。

`stopReason`は`continue`に付随し、ユーザーに表示される理由を示し、Claudeには表示されません。

#### `PreToolUse`決定制御

`PreToolUse` hooksはツール呼び出しが進行するかどうかを制御できます。

* `"allow"`は許可システムをバイパスします。`permissionDecisionReason`はユーザーに表示されますが、Claudeには表示されません。
* `"deny"`はツール呼び出しの実行を防ぎます。`permissionDecisionReason`はClaudeに表示されます。
* `"ask"`はUIでツール呼び出しを確認するようユーザーに求めます。`permissionDecisionReason`はユーザーに表示されますが、Claudeには表示されません。

さらに、hooksは`updatedInput`を使用して実行前にツール入力を変更できます：

* `updatedInput`はツールが実行される前にツールの入力パラメータを変更します
* `"permissionDecision": "allow"`と組み合わせて、入力を変更し、ツール呼び出しを自動承認します
* `"permissionDecision": "ask"`と組み合わせて、入力を変更し、確認のためにユーザーに表示します

```json  theme={null}
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",
    "permissionDecisionReason": "My reason here",
    "updatedInput": {
      "field_to_modify": "new value"
    }
  }
}
```

<Note>
  `decision`および`reason`フィールドはPreToolUse hooksでは非推奨です。
  代わりに`hookSpecificOutput.permissionDecision`および
  `hookSpecificOutput.permissionDecisionReason`を使用してください。非推奨フィールド
  `"approve"`および`"block"`は`"allow"`および`"deny"`にマップされます。
</Note>

#### `PermissionRequest`決定制御

`PermissionRequest` hooksはユーザーに表示される許可リクエストを許可または拒否できます。

* `"behavior": "allow"`の場合、オプションで`"updatedInput"`を渡すことができます。これはツールが実行される前にツールの入力パラメータを変更します。
* `"behavior": "deny"`の場合、オプションで許可が拒否された理由をモデルに伝える`"message"`文字列と、Claudeを停止する`"interrupt"`ブール値を渡すことができます。

```json  theme={null}
{
  "hookSpecificOutput": {
    "hookEventName": "PermissionRequest",
    "decision": {
      "behavior": "allow",
      "updatedInput": {
        "command": "npm run lint"
      }
    }
  }
}
```

#### `PostToolUse`決定制御

`PostToolUse` hooksはツール実行後にClaudeにフィードバックを提供できます。

* `"block"`は自動的に`reason`でClaudeにプロンプトを表示します。
* `undefined`は何もしません。`reason`は無視されます。
* `"hookSpecificOutput.additionalContext"`はClaudeが考慮するコンテキストを追加します。

```json  theme={null}
{
  "decision": "block" | undefined,
  "reason": "Explanation for decision",
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": "Additional information for Claude"
  }
}
```

#### `UserPromptSubmit`決定制御

`UserPromptSubmit` hooksはユーザープロンプトが処理されるかどうかを制御し、コンテキストを追加できます。

**コンテキストを追加する（終了コード0）**：
会話にコンテキストを追加する方法は2つあります：

1. **プレーンテキストstdout**（より簡単）：stdoutに書き込まれた非JSONテキストはコンテキストとして追加されます。これは情報を注入する最も簡単な方法です。

2. **`additionalContext`を使用したJSON**（構造化）：より多くの制御のために以下のJSON形式を使用してください。`additionalContext`フィールドはコンテキストとして追加されます。

両方の方法は終了コード0で機能します。プレーンstdoutはトランスクリプトでhook出力として表示されます。`additionalContext`はより控えめに追加されます。

**プロンプトをブロックする**：

* `"decision": "block"`はプロンプトが処理されるのを防ぎます。送信されたプロンプトはコンテキストから消去されます。`"reason"`はユーザーに表示されますが、コンテキストに追加されません。
* `"decision": undefined`（または省略）はプロンプトが通常通り進行することを許可します。

```json  theme={null}
{
  "decision": "block" | undefined,
  "reason": "Explanation for decision",
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "My additional context here"
  }
}
```

<Note>
  JSON形式は単純なユースケースでは必須ではありません。コンテキストを追加するには、終了コード0でstdoutにプレーンテキストを出力できます。プロンプトをブロックしたい場合、またはより構造化された制御が必要な場合はJSONを使用してください。
</Note>

#### `Stop`/`SubagentStop`決定制御

`Stop`および`SubagentStop` hooksはClaudeが続行する必要があるかどうかを制御できます。

* `"block"`はClaudeが停止するのを防ぎます。Claudeが続行する方法を知るために、`reason`を入力する必要があります。
* `undefined`はClaudeが停止することを許可します。`reason`は無視されます。

```json  theme={null}
{
  "decision": "block" | undefined,
  "reason": "Must be provided when Claude is blocked from stopping"
}
```

#### `SessionStart`決定制御

`SessionStart` hooksはセッションの開始時にコンテキストを読み込むことを許可します。

* `"hookSpecificOutput.additionalContext"`は文字列をコンテキストに追加します。
* 複数のhooksの`additionalContext`値は連結されます。

```json  theme={null}
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "My additional context here"
  }
}
```

#### `SessionEnd`決定制御

`SessionEnd` hooksはセッションが終了するときに実行されます。セッション終了をブロックすることはできませんが、クリーンアップタスクを実行できます。

#### 終了コード例：Bashコマンド検証

```python  theme={null}
#!/usr/bin/env python3
import json
import re
import sys

# Define validation rules as a list of (regex pattern, message) tuples
VALIDATION_RULES = [
    (
        r"\bgrep\b(?!.*\|)",
        "Use 'rg' (ripgrep) instead of 'grep' for better performance and features",
    ),
    (
        r"\bfind\s+\S+\s+-name\b",
        "Use 'rg --files | rg pattern' or 'rg --files -g pattern' instead of 'find -name' for better performance",
    ),
]


def validate_command(command: str) -> list[str]:
    issues = []
    for pattern, message in VALIDATION_RULES:
        if re.search(pattern, command):
            issues.append(message)
    return issues


try:
    input_data = json.load(sys.stdin)
except json.JSONDecodeError as e:
    print(f"Error: Invalid JSON input: {e}", file=sys.stderr)
    sys.exit(1)

tool_name = input_data.get("tool_name", "")
tool_input = input_data.get("tool_input", {})
command = tool_input.get("command", "")

if tool_name != "Bash" or not command:
    sys.exit(1)

# Validate the command
issues = validate_command(command)

if issues:
    for message in issues:
        print(f"• {message}", file=sys.stderr)
    # Exit code 2 blocks tool call and shows stderr to Claude
    sys.exit(2)
```

#### JSON出力例：コンテキストと検証を追加するUserPromptSubmit

<Note>
  `UserPromptSubmit` hooksの場合、以下の2つの方法のいずれかを使用してコンテキストを注入できます：

  * **終了コード0でのプレーンテキストstdout**：最も簡単なアプローチ、テキストを出力
  * **終了コード0でのJSON出力**：プロンプトを拒否するために`"decision": "block"`を使用するか、構造化されたコンテキスト注入のために`additionalContext`を使用

  リマインダー：終了コード2は`stderr`のみをエラーメッセージとして使用します。JSONを使用してブロックする場合（カスタム理由付き）、終了コード0で`"decision": "block"`を使用してください。
</Note>

```python  theme={null}
#!/usr/bin/env python3
import json
import sys
import re
import datetime

# Load input from stdin
try:
    input_data = json.load(sys.stdin)
except json.JSONDecodeError as e:
    print(f"Error: Invalid JSON input: {e}", file=sys.stderr)
    sys.exit(1)

prompt = input_data.get("prompt", "")

# Check for sensitive patterns
sensitive_patterns = [
    (r"(?i)\b(password|secret|key|token)\s*[:=]", "Prompt contains potential secrets"),
]

for pattern, message in sensitive_patterns:
    if re.search(pattern, prompt):
        # Use JSON output to block with a specific reason
        output = {
            "decision": "block",
            "reason": f"Security policy violation: {message}. Please rephrase your request without sensitive information."
        }
        print(json.dumps(output))
        sys.exit(0)

# Add current time to context
context = f"Current time: {datetime.datetime.now()}"
print(context)

"""
The following is also equivalent:
print(json.dumps({
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": context,
  },
}))
"""

# Allow the prompt to proceed with the additional context
sys.exit(0)
```

#### JSON出力例：承認を使用したPreToolUse

```python  theme={null}
#!/usr/bin/env python3
import json
import sys

# Load input from stdin
try:
    input_data = json.load(sys.stdin)
except json.JSONDecodeError as e:
    print(f"Error: Invalid JSON input: {e}", file=sys.stderr)
    sys.exit(1)

tool_name = input_data.get("tool_name", "")
tool_input = input_data.get("tool_input", {})

# Example: Auto-approve file reads for documentation files
if tool_name == "Read":
    file_path = tool_input.get("file_path", "")
    if file_path.endswith((".md", ".mdx", ".txt", ".json")):
        # Use JSON output to auto-approve the tool call
        output = {
            "decision": "approve",
            "reason": "Documentation file auto-approved",
            "suppressOutput": True  # Don't show in verbose mode
        }
        print(json.dumps(output))
        sys.exit(0)

# For other cases, let the normal permission flow proceed
sys.exit(0)
```

## MCPツールの操作

Claude Code hooksは[Model Context Protocol（MCP）ツール](/ja/mcp)とシームレスに動作します。MCPサーバーがツールを提供する場合、hooksでマッチできる特別な命名パターンで表示されます。

### MCPツール命名

MCPツールは`mcp__<server>__<tool>`パターンに従います。例えば：

* `mcp__memory__create_entities` - メモリサーバーのエンティティ作成ツール
* `mcp__filesystem__read_file` - ファイルシステムサーバーのファイル読み取りツール
* `mcp__github__search_repositories` - GitHubサーバーの検索ツール

### MCPツール用のhooksの設定

特定のMCPツールまたはMCPサーバー全体をターゲットにできます：

```json  theme={null}
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "mcp__memory__.*",
        "hooks": [
          {
            "type": "command",
            "command": "echo 'Memory operation initiated' >> ~/mcp-operations.log"
          }
        ]
      },
      {
        "matcher": "mcp__.*__write.*",
        "hooks": [
          {
            "type": "command",
            "command": "/home/user/scripts/validate-mcp-write.py"
          }
        ]
      }
    ]
  }
}
```

## 例

<Tip>
  コード形式、通知、ファイル保護を含む実践的な例については、スタートガイドの[その他の例](/ja/hooks-guide#more-examples)を参照してください。
</Tip>

## セキュリティに関する考慮事項

### 免責事項

**自己責任で使用してください**：Claude Code hooksはシステム上で任意のシェルコマンドを自動的に実行します。hooksを使用することで、以下を認めます：

* 設定したコマンドについては、あなたが単独で責任を負います
* Hooksはユーザーアカウントがアクセスできるすべてのファイルを変更、削除、またはアクセスできます
* 悪意のある、または不適切に書かれたhooksはデータ損失またはシステム損害を引き起こす可能性があります
* Anthropicは保証を提供せず、hookの使用から生じるいかなる損害についても責任を負いません
* 本番環境で使用する前に、安全な環境でhooksを徹底的にテストする必要があります

hookの設定に追加する前に、すべてのhookコマンドを確認して理解してください。

### セキュリティのベストプラクティス

より安全なhooksを書くための重要なプラクティスは以下の通りです：

1. **入力を検証およびサニタイズする** - 入力データを盲目的に信頼しないでください
2. **常にシェル変数をクォートする** - `$VAR`ではなく`"$VAR"`を使用してください
3. **パストラバーサルをブロックする** - ファイルパスで`..`をチェックしてください
4. **絶対パスを使用する** - スクリプトの完全なパスを指定してください（プロジェクトパスに`"$CLAUDE_PROJECT_DIR"`を使用）
5. **機密ファイルをスキップする** - `.env`、`.git/`、キーなどを避けてください

### 設定セーフティ

設定ファイルのhooksへの直接編集は即座には有効になりません。Claude Codeは：

1. スタートアップ時にhooksのスナップショットをキャプチャします
2. セッション全体でこのスナップショットを使用します
3. hooksが外部で変更された場合に警告します
4. 変更を適用するために`/hooks`メニューでレビューが必要です

これにより、悪意のあるhook変更が現在のセッションに影響するのを防ぎます。

## Hook実行の詳細

* **タイムアウト**：デフォルトで60秒の実行制限、コマンドごとに設定可能。
  * 個別のコマンドのタイムアウトは他のコマンドに影響しません。
* **並列化**：マッチするすべてのhooksが並列で実行されます
* **重複排除**：同一のhookコマンドは自動的に重複排除されます
* **環境**：現在のディレクトリでClaude Codeの環境で実行されます
  * `CLAUDE_PROJECT_DIR`環境変数が利用可能で、プロジェクトルートディレクトリへの絶対パスが含まれます（Claude Codeが開始された場所）
  * `CLAUDE_CODE_REMOTE`環境変数はhookがリモート（web）環境（`"true"`）で実行されているか、ローカルCLI環境（設定されていないか空）で実行されているかを示します。実行コンテキストに基づいて異なるロジックを実行するために使用してください。
* **入力**：stdinを介したJSON
* **出力**：
  * PreToolUse/PermissionRequest/PostToolUse/Stop/SubagentStop：詳細モード（ctrl+o）で進捗を表示
  * Notification/SessionEnd：デバッグのみにログ（`--debug`）
  * UserPromptSubmit/SessionStart：stdoutはClaudeのコンテキストとして追加

## デバッグ

### 基本的なトラブルシューティング

hooksが機能していない場合：

1. **設定を確認** - `/hooks`を実行してhookが登録されているかを確認
2. **構文を検証** - JSON設定が有効であることを確認
3. **コマンドをテスト** - hookコマンドを最初に手動で実行
4. **権限を確認** - スクリプトが実行可能であることを確認
5. **ログを確認** - `claude --debug`を使用してhook実行の詳細を確認

一般的な問題：

* **エスケープされていないクォート** - JSON文字列内で`\"`を使用してください
* **間違ったマッチャー** - ツール名が正確にマッチすることを確認してください（大文字小文字を区別）
* **コマンドが見つからない** - スクリプトの完全なパスを使用してください

### 高度なデバッグ

複雑なhook問題の場合：

1. **hook実行を検査** - `claude --debug`を使用してhook実行の詳細を確認
2. **JSONスキーマを検証** - 外部ツールでhook入出力をテスト
3. **環境変数を確認** - Claude Codeの環境が正しいことを確認
4. **エッジケースをテスト** - 異常なファイルパスまたは入力でhooksを試す
5. **システムリソースを監視** - hook実行中のリソース枯渇をチェック
6. **構造化ログを使用** - hookスクリプトにログを実装

### デバッグ出力例

`claude --debug`を使用してhook実行の詳細を確認：

```
[DEBUG] Executing hooks for PostToolUse:Write
[DEBUG] Getting matching hook commands for PostToolUse with query: Write
[DEBUG] Found 1 hook matchers in settings
[DEBUG] Matched 1 hooks for query "Write"
[DEBUG] Found 1 hook commands to execute
[DEBUG] Executing hook command: <Your command> with timeout 60000ms
[DEBUG] Hook command completed with status 0: <Your stdout>
```

進捗メッセージは詳細モード（ctrl+o）で表示されます：

* 実行中のhook
* 実行されるコマンド
* 成功/失敗ステータス
* 出力またはエラーメッセージ


---

> To find navigation and other pages in this documentation, fetch the llms.txt file at: https://code.claude.com/docs/llms.txt