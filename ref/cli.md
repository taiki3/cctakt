# CLIリファレンス

> Claude Codeのコマンドラインインターフェースの完全なリファレンス（コマンドとフラグを含む）。

## CLIコマンド

| コマンド                            | 説明                                 | 例                                            |
| :------------------------------ | :--------------------------------- | :------------------------------------------- |
| `claude`                        | インタラクティブREPLを開始                    | `claude`                                     |
| `claude "query"`                | 初期プロンプト付きでREPLを開始                  | `claude "explain this project"`              |
| `claude -p "query"`             | SDKを経由してクエリを実行してから終了               | `claude -p "explain this function"`          |
| `cat file \| claude -p "query"` | パイプされたコンテンツを処理                     | `cat logs.txt \| claude -p "explain"`        |
| `claude -c`                     | 現在のディレクトリで最新の会話を続行                 | `claude -c`                                  |
| `claude -c -p "query"`          | SDKを経由して続行                         | `claude -c -p "Check for type errors"`       |
| `claude -r "<session>" "query"` | セッションIDまたは名前でセッションを再開              | `claude -r "auth-refactor" "Finish this PR"` |
| `claude update`                 | 最新バージョンに更新                         | `claude update`                              |
| `claude mcp`                    | Model Context Protocol（MCP）サーバーを設定 | [Claude Code MCPドキュメント](/ja/mcp)を参照してください。   |

## CLIフラグ

これらのコマンドラインフラグを使用してClaude Codeの動作をカスタマイズします：

| フラグ                              | 説明                                                                                                                                           | 例                                                                                                  |
| :------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------- |
| `--add-dir`                      | Claudeがアクセスできる追加の作業ディレクトリを追加します（各パスがディレクトリとして存在することを検証します）                                                                                   | `claude --add-dir ../apps ../lib`                                                                  |
| `--agent`                        | 現在のセッション用のエージェントを指定します（`agent`設定をオーバーライドします）                                                                                                 | `claude --agent my-custom-agent`                                                                   |
| `--agents`                       | カスタム[サブエージェント](/ja/sub-agents)をJSONで動的に定義します（形式については以下を参照）                                                                                   | `claude --agents '{"reviewer":{"description":"Reviews code","prompt":"You are a code reviewer"}}'` |
| `--allowedTools`                 | 許可を求めずに実行するツール。利用可能なツールを制限するには、代わりに`--tools`を使用してください                                                                                        | `"Bash(git log:*)" "Bash(git diff:*)" "Read"`                                                      |
| `--append-system-prompt`         | デフォルトシステムプロンプトの末尾にカスタムテキストを追加します（インタラクティブモードと印刷モードの両方で機能します）                                                                                 | `claude --append-system-prompt "Always use TypeScript"`                                            |
| `--betas`                        | APIリクエストに含めるベータヘッダー（APIキーユーザーのみ）                                                                                                             | `claude --betas interleaved-thinking`                                                              |
| `--chrome`                       | Webオートメーションとテスト用の[Chromeブラウザ統合](/ja/chrome)を有効にします                                                                                           | `claude --chrome`                                                                                  |
| `--continue`, `-c`               | 現在のディレクトリで最新の会話を読み込みます                                                                                                                       | `claude --continue`                                                                                |
| `--dangerously-skip-permissions` | 許可プロンプトをスキップします（注意して使用してください）                                                                                                                | `claude --dangerously-skip-permissions`                                                            |
| `--debug`                        | デバッグモードを有効にします（オプションでカテゴリフィルタリング可能。例：`"api,hooks"`または`"!statsig,!file"`）                                                                     | `claude --debug "api,mcp"`                                                                         |
| `--disallowedTools`              | モデルのコンテキストから削除され、使用できないツール                                                                                                                   | `"Bash(git log:*)" "Bash(git diff:*)" "Edit"`                                                      |
| `--fallback-model`               | デフォルトモデルがオーバーロードされた場合、指定されたモデルへの自動フォールバックを有効にします（印刷モードのみ）                                                                                    | `claude -p --fallback-model sonnet "query"`                                                        |
| `--fork-session`                 | 再開時に、元のセッションIDを再利用する代わりに新しいセッションIDを作成します（`--resume`または`--continue`と一緒に使用）                                                                    | `claude --resume abc123 --fork-session`                                                            |
| `--ide`                          | 起動時に、正確に1つの有効なIDEが利用可能な場合、自動的にIDEに接続します                                                                                                      | `claude --ide`                                                                                     |
| `--include-partial-messages`     | 部分的なストリーミングイベントを出力に含めます（`--print`と`--output-format=stream-json`が必要）                                                                          | `claude -p --output-format stream-json --include-partial-messages "query"`                         |
| `--input-format`                 | 印刷モード用の入力形式を指定します（オプション：`text`、`stream-json`）                                                                                                | `claude -p --output-format json --input-format stream-json`                                        |
| `--json-schema`                  | エージェントがワークフローを完了した後、JSONスキーマに一致する検証済みJSON出力を取得します（印刷モードのみ、[Agent SDK構造化出力](https://docs.claude.com/en/docs/agent-sdk/structured-outputs)を参照） | `claude -p --json-schema '{"type":"object","properties":{...}}' "query"`                           |
| `--max-turns`                    | エージェンティックターンの数を制限します（印刷モードのみ）。制限に達するとエラーで終了します。デフォルトでは制限なし                                                                                   | `claude -p --max-turns 3 "query"`                                                                  |
| `--mcp-config`                   | MCPサーバーをJSONファイルまたは文字列から読み込みます（スペース区切り）                                                                                                      | `claude --mcp-config ./mcp.json`                                                                   |
| `--model`                        | 現在のセッション用のモデルを設定します。最新モデルのエイリアス（`sonnet`または`opus`）またはモデルの完全な名前を使用                                                                            | `claude --model claude-sonnet-4-5-20250929`                                                        |
| `--no-chrome`                    | このセッション用の[Chromeブラウザ統合](/ja/chrome)を無効にします                                                                                                   | `claude --no-chrome`                                                                               |
| `--output-format`                | 印刷モード用の出力形式を指定します（オプション：`text`、`json`、`stream-json`）                                                                                         | `claude -p "query" --output-format json`                                                           |
| `--permission-mode`              | 指定された[許可モード](/ja/iam#permission-modes)で開始します                                                                                                 | `claude --permission-mode plan`                                                                    |
| `--permission-prompt-tool`       | 非インタラクティブモードで許可プロンプトを処理するMCPツールを指定します                                                                                                        | `claude -p --permission-prompt-tool mcp_auth_tool "query"`                                         |
| `--plugin-dir`                   | このセッションのみプラグインをディレクトリから読み込みます（繰り返し可能）                                                                                                        | `claude --plugin-dir ./my-plugins`                                                                 |
| `--print`, `-p`                  | インタラクティブモードなしで応答を印刷します（プログラム的な使用の詳細については[SDKドキュメント](https://docs.claude.com/en/docs/agent-sdk)を参照）                                           | `claude -p "query"`                                                                                |
| `--resume`, `-r`                 | IDまたは名前で特定のセッションを再開するか、インタラクティブピッカーを表示してセッションを選択します                                                                                          | `claude --resume auth-refactor`                                                                    |
| `--session-id`                   | 会話用に特定のセッションIDを使用します（有効なUUIDである必要があります）                                                                                                      | `claude --session-id "550e8400-e29b-41d4-a716-446655440000"`                                       |
| `--setting-sources`              | 読み込む設定ソースのカンマ区切りリスト（`user`、`project`、`local`）                                                                                                | `claude --setting-sources user,project`                                                            |
| `--settings`                     | 追加の設定を読み込むための設定JSONファイルまたはJSON文字列へのパス                                                                                                        | `claude --settings ./settings.json`                                                                |
| `--strict-mcp-config`            | `--mcp-config`からのMCPサーバーのみを使用し、他のすべてのMCP設定を無視します                                                                                             | `claude --strict-mcp-config --mcp-config ./mcp.json`                                               |
| `--system-prompt`                | デフォルトシステムプロンプト全体をカスタムテキストに置き換えます（インタラクティブモードと印刷モードの両方で機能します）                                                                                 | `claude --system-prompt "You are a Python expert"`                                                 |
| `--system-prompt-file`           | ファイルからシステムプロンプトを読み込み、デフォルトプロンプトを置き換えます（印刷モードのみ）                                                                                              | `claude -p --system-prompt-file ./custom-prompt.txt "query"`                                       |
| `--tools`                        | Claudeが使用できる組み込みツールを制限します（インタラクティブモードと印刷モードの両方で機能します）。すべてを無効にするには`""`を、すべてを有効にするには`"default"`を、または`"Bash,Edit,Read"`のようなツール名を使用              | `claude --tools "Bash,Edit,Read"`                                                                  |
| `--verbose`                      | 詳細ログを有効にし、ターンバイターンの完全な出力を表示します（印刷モードとインタラクティブモードの両方でデバッグに役立ちます）                                                                              | `claude --verbose`                                                                                 |
| `--version`, `-v`                | バージョン番号を出力                                                                                                                                   | `claude -v`                                                                                        |

<Tip>
  `--output-format json`フラグは、スクリプトとオートメーションに特に役立ち、
  Claudeの応答をプログラム的に解析できます。
</Tip>

### エージェントフラグ形式

`--agents`フラグは、1つ以上のカスタムサブエージェントを定義するJSONオブジェクトを受け入れます。各サブエージェントには、一意の名前（キーとして）と、以下のフィールドを持つ定義オブジェクトが必要です：

| フィールド         | 必須  | 説明                                                                         |
| :------------ | :-- | :------------------------------------------------------------------------- |
| `description` | はい  | サブエージェントを呼び出すべき時期の自然言語説明                                                   |
| `prompt`      | はい  | サブエージェントの動作を導くシステムプロンプト                                                    |
| `tools`       | いいえ | サブエージェントが使用できる特定のツールの配列（例：`["Read", "Edit", "Bash"]`）。省略した場合、すべてのツールを継承します |
| `model`       | いいえ | 使用するモデルエイリアス：`sonnet`、`opus`、または`haiku`。省略した場合、デフォルトのサブエージェントモデルを使用します     |

例：

```bash  theme={null}
claude --agents '{
  "code-reviewer": {
    "description": "Expert code reviewer. Use proactively after code changes.",
    "prompt": "You are a senior code reviewer. Focus on code quality, security, and best practices.",
    "tools": ["Read", "Grep", "Glob", "Bash"],
    "model": "sonnet"
  },
  "debugger": {
    "description": "Debugging specialist for errors and test failures.",
    "prompt": "You are an expert debugger. Analyze errors, identify root causes, and provide fixes."
  }
}'
```

サブエージェントの作成と使用の詳細については、[サブエージェントドキュメント](/ja/sub-agents)を参照してください。

### システムプロンプトフラグ

Claude Codeは、システムプロンプトをカスタマイズするための3つのフラグを提供し、それぞれ異なる目的に機能します：

| フラグ                      | 動作                    | モード           | ユースケース                             |
| :----------------------- | :-------------------- | :------------ | :--------------------------------- |
| `--system-prompt`        | **デフォルトプロンプト全体を置き換え** | インタラクティブ + 印刷 | Claudeの動作と指示を完全に制御                 |
| `--system-prompt-file`   | **ファイルコンテンツで置き換え**    | 印刷のみ          | 再現性とバージョン管理のためにファイルからプロンプトを読み込み    |
| `--append-system-prompt` | **デフォルトプロンプトに追加**     | インタラクティブ + 印刷 | デフォルトのClaude Code動作を保持しながら特定の指示を追加 |

**各フラグを使用する時期：**

* **`--system-prompt`**: Claude のシステムプロンプトを完全に制御する必要がある場合に使用します。これにより、すべてのデフォルトClaude Code指示が削除され、白紙の状態が得られます。
  ```bash  theme={null}
  claude --system-prompt "You are a Python expert who only writes type-annotated code"
  ```

* **`--system-prompt-file`**: ファイルからカスタムプロンプトを読み込みたい場合に使用します。チームの一貫性またはバージョン管理されたプロンプトテンプレートに役立ちます。
  ```bash  theme={null}
  claude -p --system-prompt-file ./prompts/code-review.txt "Review this PR"
  ```

* **`--append-system-prompt`**: Claude Codeのデフォルト機能を保持しながら特定の指示を追加したい場合に使用します。これはほとんどのユースケースで最も安全なオプションです。
  ```bash  theme={null}
  claude --append-system-prompt "Always use TypeScript and include JSDoc comments"
  ```

<Note>
  `--system-prompt`と`--system-prompt-file`は相互に排他的です。両方のフラグを同時に使用することはできません。
</Note>

<Tip>
  ほとんどのユースケースでは、Claude Codeの組み込み機能を保持しながらカスタム要件を追加するため、`--append-system-prompt`が推奨されます。システムプロンプトを完全に制御する必要がある場合のみ、`--system-prompt`または`--system-prompt-file`を使用してください。
</Tip>

出力形式、ストリーミング、詳細ログ、プログラム的な使用を含む印刷モード（`-p`）の詳細については、
[SDKドキュメント](https://docs.claude.com/en/docs/agent-sdk)を参照してください。

## 関連項目

* [Chrome拡張機能](/ja/chrome) - ブラウザオートメーションとWebテスト
* [インタラクティブモード](/ja/interactive-mode) - ショートカット、入力モード、インタラクティブ機能
* [スラッシュコマンド](/ja/slash-commands) - インタラクティブセッションコマンド
* [クイックスタートガイド](/ja/quickstart) - Claude Codeの開始
* [一般的なワークフロー](/ja/common-workflows) - 高度なワークフローとパターン
* [設定](/ja/settings) - 設定オプション
* [SDKドキュメント](https://docs.claude.com/en/docs/agent-sdk) - プログラム的な使用と統合


---

> To find navigation and other pages in this documentation, fetch the llms.txt file at: https://code.claude.com/docs/llms.txt