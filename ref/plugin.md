# プラグインリファレンス

> Claude Codeプラグインシステムの完全な技術リファレンス。スキーマ、CLIコマンド、コンポーネント仕様を含みます。

<Tip>
  プラグインをインストールしたいですか？[プラグインの発見とインストール](/ja/discover-plugins)を参照してください。プラグインの作成については、[プラグイン](/ja/plugins)を参照してください。プラグインの配布については、[プラグインマーケットプレイス](/ja/plugin-marketplaces)を参照してください。
</Tip>

このリファレンスは、Claude Codeプラグインシステムの完全な技術仕様を提供します。コンポーネントスキーマ、CLIコマンド、開発ツールを含みます。

## プラグインコンポーネントリファレンス

このセクションでは、プラグインが提供できる5つのタイプのコンポーネントについて説明します。

### コマンド

プラグインは、Claude Codeのコマンドシステムとシームレスに統合されるカスタムスラッシュコマンドを追加します。

**場所**: プラグインルートの`commands/`ディレクトリ

**ファイル形式**: フロントマター付きのMarkdownファイル

プラグインコマンド構造、呼び出しパターン、機能の詳細については、[プラグインコマンド](/ja/slash-commands#plugin-commands)を参照してください。

### エージェント

プラグインは、特定のタスク用の専門的なサブエージェントを提供でき、Claude が必要に応じて自動的に呼び出すことができます。

**場所**: プラグインルートの`agents/`ディレクトリ

**ファイル形式**: エージェント機能を説明するMarkdownファイル

**エージェント構造**:

```markdown  theme={null}
---
description: このエージェントが専門とする内容
capabilities: ["task1", "task2", "task3"]
---

# エージェント名

エージェントの役割、専門知識、およびClaudeがそれを呼び出すべき時期の詳細な説明。

## 機能
- エージェントが得意とする特定のタスク
- もう1つの専門的な機能
- このエージェントと他のエージェントを使い分ける時期

## コンテキストと例
このエージェントを使用すべき時期と、それが解決する問題の種類の例を提供します。
```

**統合ポイント**:

* エージェントは`/agents`インターフェイスに表示されます
* Claudeはタスクコンテキストに基づいてエージェントを自動的に呼び出すことができます
* エージェントはユーザーによって手動で呼び出すことができます
* プラグインエージェントは組み込みのClaudeエージェントと一緒に機能します

### スキル

プラグインは、Claudeの機能を拡張するエージェントスキルを提供できます。スキルはモデル呼び出し型です。Claudeはタスクコンテキストに基づいて自動的に使用するかどうかを決定します。

**場所**: プラグインルートの`skills/`ディレクトリ

**ファイル形式**: フロントマター付きの`SKILL.md`ファイルを含むディレクトリ

**スキル構造**:

```
skills/
├── pdf-processor/
│   ├── SKILL.md
│   ├── reference.md (オプション)
│   └── scripts/ (オプション)
└── code-reviewer/
    └── SKILL.md
```

**統合動作**:

* プラグインスキルはプラグインがインストールされると自動的に検出されます
* Claudeはマッチするタスクコンテキストに基づいてスキルを自動的に呼び出します
* スキルはSKILL.mdの隣にサポートファイルを含めることができます

SKILL.md形式とスキル作成の完全なガイダンスについては、以下を参照してください:

* [Claude CodeでスキルをUse](/ja/skills)
* [エージェントスキル概要](https://docs.claude.com/en/docs/agents-and-tools/agent-skills/overview#skill-structure)

### フック

プラグインは、Claude Codeイベントに自動的に応答するイベントハンドラーを提供できます。

**場所**: プラグインルートの`hooks/hooks.json`、またはplugin.jsonにインライン

**形式**: イベントマッチャーとアクションを含むJSON設定

**フック設定**:

```json  theme={null}
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/scripts/format-code.sh"
          }
        ]
      }
    ]
  }
}
```

**利用可能なイベント**:

* `PreToolUse`: Claudeがツールを使用する前
* `PostToolUse`: Claudeがツールを正常に使用した後
* `PostToolUseFailure`: Claudeのツール実行が失敗した後
* `PermissionRequest`: パーミッションダイアログが表示されたとき
* `UserPromptSubmit`: ユーザーがプロンプトを送信したとき
* `Notification`: Claude Codeが通知を送信するとき
* `Stop`: Claudeが停止しようとするとき
* `SubagentStart`: サブエージェントが開始されたとき
* `SubagentStop`: サブエージェントが停止しようとするとき
* `SessionStart`: セッションの開始時
* `SessionEnd`: セッションの終了時
* `PreCompact`: 会話履歴がコンパクト化される前

**フックタイプ**:

* `command`: シェルコマンドまたはスクリプトを実行
* `prompt`: LLMでプロンプトを評価（コンテキスト用に`$ARGUMENTS`プレースホルダーを使用）
* `agent`: 複雑な検証タスク用にツールを備えたエージェント検証器を実行

### MCPサーバー

プラグインは、Model Context Protocol（MCP）サーバーをバンドルして、Claude Codeを外部ツールおよびサービスに接続できます。

**場所**: プラグインルートの`.mcp.json`、またはplugin.jsonにインライン

**形式**: 標準MCPサーバー設定

**MCPサーバー設定**:

```json  theme={null}
{
  "mcpServers": {
    "plugin-database": {
      "command": "${CLAUDE_PLUGIN_ROOT}/servers/db-server",
      "args": ["--config", "${CLAUDE_PLUGIN_ROOT}/config.json"],
      "env": {
        "DB_PATH": "${CLAUDE_PLUGIN_ROOT}/data"
      }
    },
    "plugin-api-client": {
      "command": "npx",
      "args": ["@company/mcp-server", "--plugin-mode"],
      "cwd": "${CLAUDE_PLUGIN_ROOT}"
    }
  }
}
```

**統合動作**:

* プラグインMCPサーバーはプラグインが有効になると自動的に開始されます
* サーバーはClaudeのツールキットに標準MCPツールとして表示されます
* サーバー機能はClaudeの既存ツールとシームレスに統合されます
* プラグインサーバーはユーザーMCPサーバーとは独立して設定できます

### LSPサーバー

<Tip>
  LSPプラグインを使用したいですか？公式マーケットプレイスからインストールしてください。`/plugin`の「Discover」タブで「lsp」を検索してください。このセクションでは、公式マーケットプレイスでカバーされていない言語用のLSPプラグインを作成する方法について説明します。
</Tip>

プラグインは、[Language Server Protocol](https://microsoft.github.io/language-server-protocol/)（LSP）サーバーを提供して、Claudeがコードベースで作業中にリアルタイムコードインテリジェンスを取得できます。

LSP統合は以下を提供します:

* **インスタント診断**: Claudeは各編集後すぐにエラーと警告を表示します
* **コードナビゲーション**: 定義へのジャンプ、参照の検索、ホバー情報
* **言語認識**: コードシンボルの型情報とドキュメント

**場所**: プラグインルートの`.lsp.json`、またはplugin.jsonにインライン

**形式**: 言語サーバー名をその設定にマップするJSON設定

**`.lsp.json`ファイル形式**:

```json  theme={null}
{
  "go": {
    "command": "gopls",
    "args": ["serve"],
    "extensionToLanguage": {
      ".go": "go"
    }
  }
}
```

**`plugin.json`にインライン**:

```json  theme={null}
{
  "name": "my-plugin",
  "lspServers": {
    "go": {
      "command": "gopls",
      "args": ["serve"],
      "extensionToLanguage": {
        ".go": "go"
      }
    }
  }
}
```

**必須フィールド:**

| フィールド                 | 説明                              |
| :-------------------- | :------------------------------ |
| `command`             | 実行するLSPバイナリ（PATHに含まれている必要があります） |
| `extensionToLanguage` | ファイル拡張子を言語識別子にマップ               |

**オプションフィールド:**

| フィールド                   | 説明                                          |
| :---------------------- | :------------------------------------------ |
| `args`                  | LSPサーバーのコマンドライン引数                           |
| `transport`             | 通信トランスポート: `stdio`（デフォルト）または`socket`        |
| `env`                   | サーバー起動時に設定する環境変数                            |
| `initializationOptions` | 初期化中にサーバーに渡されるオプション                         |
| `settings`              | `workspace/didChangeConfiguration`経由で渡される設定 |
| `workspaceFolder`       | サーバーのワークスペースフォルダパス                          |
| `startupTimeout`        | サーバー起動を待つ最大時間（ミリ秒）                          |
| `shutdownTimeout`       | グレースフルシャットダウンを待つ最大時間（ミリ秒）                   |
| `restartOnCrash`        | サーバーがクラッシュした場合に自動的に再起動するかどうか                |
| `maxRestarts`           | 諦める前の最大再起動試行回数                              |

<Warning>
  **言語サーバーバイナリを別途インストールする必要があります。** LSPプラグインはClaude Codeが言語サーバーに接続する方法を設定しますが、サーバー自体は含まれていません。`/plugin`の「Errors」タブに`Executable not found in $PATH`が表示される場合は、言語用の必要なバイナリをインストールしてください。
</Warning>

**利用可能なLSPプラグイン:**

| プラグイン            | 言語サーバー                     | インストールコマンド                                                                          |
| :--------------- | :------------------------- | :---------------------------------------------------------------------------------- |
| `pyright-lsp`    | Pyright（Python）            | `pip install pyright`または`npm install -g pyright`                                    |
| `typescript-lsp` | TypeScript Language Server | `npm install -g typescript-language-server typescript`                              |
| `rust-lsp`       | rust-analyzer              | [rust-analyzer インストールを参照](https://rust-analyzer.github.io/manual.html#installation) |

言語サーバーをインストールしてから、マーケットプレイスからプラグインをインストールしてください。

***

## プラグインインストールスコープ

プラグインをインストールするときは、プラグインが利用可能な場所と他のユーザーが使用できるかどうかを決定する**スコープ**を選択します:

| スコープ      | 設定ファイル                        | ユースケース                           |
| :-------- | :---------------------------- | :------------------------------- |
| `user`    | `~/.claude/settings.json`     | すべてのプロジェクト全体で利用可能な個人プラグイン（デフォルト） |
| `project` | `.claude/settings.json`       | バージョン管理経由で共有されるチームプラグイン          |
| `local`   | `.claude/settings.local.json` | プロジェクト固有のプラグイン、gitignored        |
| `managed` | `managed-settings.json`       | 管理されたプラグイン（読み取り専用、更新のみ）          |

プラグインは他のClaude Code設定と同じスコープシステムを使用します。インストール手順とスコープフラグについては、[プラグインのインストール](/ja/discover-plugins#install-plugins)を参照してください。スコープの完全な説明については、[設定スコープ](/ja/settings#configuration-scopes)を参照してください。

***

## プラグインマニフェストスキーマ

`plugin.json`ファイルはプラグインのメタデータと設定を定義します。このセクションでは、サポートされているすべてのフィールドとオプションについて説明します。

### 完全なスキーマ

```json  theme={null}
{
  "name": "plugin-name",
  "version": "1.2.0",
  "description": "Brief plugin description",
  "author": {
    "name": "Author Name",
    "email": "author@example.com",
    "url": "https://github.com/author"
  },
  "homepage": "https://docs.example.com/plugin",
  "repository": "https://github.com/author/plugin",
  "license": "MIT",
  "keywords": ["keyword1", "keyword2"],
  "commands": ["./custom/commands/special.md"],
  "agents": "./custom/agents/",
  "skills": "./custom/skills/",
  "hooks": "./config/hooks.json",
  "mcpServers": "./mcp-config.json",
  "outputStyles": "./styles/",
  "lspServers": "./.lsp.json"
}
```

### 必須フィールド

| フィールド  | 型      | 説明                    | 例                    |
| :----- | :----- | :-------------------- | :------------------- |
| `name` | string | 一意の識別子（ケバブケース、スペースなし） | `"deployment-tools"` |

### メタデータフィールド

| フィールド         | 型      | 説明            | 例                                                  |
| :------------ | :----- | :------------ | :------------------------------------------------- |
| `version`     | string | セマンティックバージョン  | `"2.1.0"`                                          |
| `description` | string | プラグイン目的の簡潔な説明 | `"Deployment automation tools"`                    |
| `author`      | object | 著者情報          | `{"name": "Dev Team", "email": "dev@company.com"}` |
| `homepage`    | string | ドキュメントURL     | `"https://docs.example.com"`                       |
| `repository`  | string | ソースコードURL     | `"https://github.com/user/plugin"`                 |
| `license`     | string | ライセンス識別子      | `"MIT"`、`"Apache-2.0"`                             |
| `keywords`    | array  | 検出タグ          | `["deployment", "ci-cd"]`                          |

### コンポーネントパスフィールド

| フィールド          | 型              | 説明                                                                                                         | 例                                     |
| :------------- | :------------- | :--------------------------------------------------------------------------------------------------------- | :------------------------------------ |
| `commands`     | string\|array  | 追加のコマンドファイル/ディレクトリ                                                                                         | `"./custom/cmd.md"`または`["./cmd1.md"]` |
| `agents`       | string\|array  | 追加のエージェントファイル                                                                                              | `"./custom/agents/"`                  |
| `skills`       | string\|array  | 追加のスキルディレクトリ                                                                                               | `"./custom/skills/"`                  |
| `hooks`        | string\|object | フック設定パスまたはインライン設定                                                                                          | `"./hooks.json"`                      |
| `mcpServers`   | string\|object | MCP設定パスまたはインライン設定                                                                                          | `"./mcp-config.json"`                 |
| `outputStyles` | string\|array  | 追加の出力スタイルファイル/ディレクトリ                                                                                       | `"./styles/"`                         |
| `lspServers`   | string\|object | [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)コード知能設定（定義へのジャンプ、参照の検索など） | `"./.lsp.json"`                       |

### パス動作ルール

**重要**: カスタムパスはデフォルトディレクトリを置き換えるのではなく、補足します。

* `commands/`が存在する場合、カスタムコマンドパスに加えてロードされます
* すべてのパスはプラグインルートに対して相対的で、`./`で始まる必要があります
* カスタムパスのコマンドは同じ命名とネームスペーシングルールを使用します
* 複数のパスを配列として指定して柔軟性を持たせることができます

**パスの例**:

```json  theme={null}
{
  "commands": [
    "./specialized/deploy.md",
    "./utilities/batch-process.md"
  ],
  "agents": [
    "./custom-agents/reviewer.md",
    "./custom-agents/tester.md"
  ]
}
```

### 環境変数

**`${CLAUDE_PLUGIN_ROOT}`**: プラグインディレクトリへの絶対パスを含みます。フック、MCPサーバー、スクリプトで使用して、インストール場所に関係なく正しいパスを確保します。

```json  theme={null}
{
  "hooks": {
    "PostToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/scripts/process.sh"
          }
        ]
      }
    ]
  }
}
```

***

## プラグインキャッシングとファイル解決

セキュリティと検証の目的で、Claude Codeはプラグインをインプレイスで使用するのではなく、キャッシュディレクトリにコピーします。プラグインの開発時に外部ファイルを参照する場合、この動作を理解することが重要です。

### プラグインキャッシングの仕組み

プラグインをインストールすると、Claude Codeはプラグインファイルをキャッシュディレクトリにコピーします:

* **相対パスを持つマーケットプレイスプラグインの場合**: `source`フィールドで指定されたパスが再帰的にコピーされます。たとえば、マーケットプレイスエントリが`"source": "./plugins/my-plugin"`を指定している場合、`./plugins`ディレクトリ全体がコピーされます。
* **`.claude-plugin/plugin.json`を持つプラグインの場合**: 暗黙的なルートディレクトリ（`.claude-plugin/plugin.json`を含むディレクトリ）が再帰的にコピーされます。

### パストラバーサルの制限

プラグインはコピーされたディレクトリ構造の外のファイルを参照できません。プラグインルートの外を走査するパス（`../shared-utils`など）は、インストール後に機能しません。これらの外部ファイルはキャッシュにコピーされないためです。

### 外部依存関係の処理

プラグインがディレクトリの外のファイルにアクセスする必要がある場合、2つのオプションがあります:

**オプション1: シンボリックリンクを使用**

プラグインディレクトリ内の外部ファイルへのシンボリックリンクを作成します。シンボリックリンクはコピープロセス中に尊重されます:

```bash  theme={null}
# プラグインディレクトリ内
ln -s /path/to/shared-utils ./shared-utils
```

シンボリックリンクされたコンテンツはプラグインキャッシュにコピーされます。

**オプション2: マーケットプレイスを再構成**

プラグインパスを必要なすべてのファイルを含む親ディレクトリに設定し、残りのプラグインマニフェストをマーケットプレイスエントリに直接提供します:

```json  theme={null}
{
  "name": "my-plugin",
  "source": "./",
  "description": "Plugin that needs root-level access",
  "commands": ["./plugins/my-plugin/commands/"],
  "agents": ["./plugins/my-plugin/agents/"],
  "strict": false
}
```

このアプローチはマーケットプレイスルート全体をコピーし、プラグインに兄弟ディレクトリへのアクセスを提供します。

<Note>
  プラグインの論理的ルートの外の場所を指すシンボリックリンクはコピー中にフォローされます。これはキャッシングシステムのセキュリティ利点を維持しながら柔軟性を提供します。
</Note>

***

## プラグインディレクトリ構造

### 標準プラグインレイアウト

完全なプラグインは次の構造に従います:

```
enterprise-plugin/
├── .claude-plugin/           # メタデータディレクトリ
│   └── plugin.json          # 必須: プラグインマニフェスト
├── commands/                 # デフォルトコマンド場所
│   ├── status.md
│   └── logs.md
├── agents/                   # デフォルトエージェント場所
│   ├── security-reviewer.md
│   ├── performance-tester.md
│   └── compliance-checker.md
├── skills/                   # エージェントスキル
│   ├── code-reviewer/
│   │   └── SKILL.md
│   └── pdf-processor/
│       ├── SKILL.md
│       └── scripts/
├── hooks/                    # フック設定
│   ├── hooks.json           # メインフック設定
│   └── security-hooks.json  # 追加フック
├── .mcp.json                # MCPサーバー定義
├── .lsp.json                # LSPサーバー設定
├── scripts/                 # フックとユーティリティスクリプト
│   ├── security-scan.sh
│   ├── format-code.py
│   └── deploy.js
├── LICENSE                  # ライセンスファイル
└── CHANGELOG.md             # バージョン履歴
```

<Warning>
  `.claude-plugin/`ディレクトリには`plugin.json`ファイルが含まれています。他のすべてのディレクトリ（commands/、agents/、skills/、hooks/）は`.claude-plugin/`の内部ではなく、プラグインルートにある必要があります。
</Warning>

### ファイル場所リファレンス

| コンポーネント     | デフォルト場所                      | 目的                       |
| :---------- | :--------------------------- | :----------------------- |
| **マニフェスト**  | `.claude-plugin/plugin.json` | 必須メタデータファイル              |
| **コマンド**    | `commands/`                  | スラッシュコマンドMarkdownファイル    |
| **エージェント**  | `agents/`                    | サブエージェントMarkdownファイル     |
| **スキル**     | `skills/`                    | SKILL.mdファイルを含むエージェントスキル |
| **フック**     | `hooks/hooks.json`           | フック設定                    |
| **MCPサーバー** | `.mcp.json`                  | MCPサーバー定義                |
| **LSPサーバー** | `.lsp.json`                  | 言語サーバー設定                 |

***

## CLIコマンドリファレンス

Claude Codeは、スクリプトと自動化に役立つ非対話的なプラグイン管理用のCLIコマンドを提供します。

### plugin install

利用可能なマーケットプレイスからプラグインをインストールします。

```bash  theme={null}
claude plugin install <plugin> [options]
```

**引数:**

* `<plugin>`: プラグイン名または特定のマーケットプレイス用の`plugin-name@marketplace-name`

**オプション:**

| オプション                 | 説明                                      | デフォルト  |
| :-------------------- | :-------------------------------------- | :----- |
| `-s, --scope <scope>` | インストールスコープ: `user`、`project`、または`local` | `user` |
| `-h, --help`          | コマンドのヘルプを表示                             |        |

**例:**

```bash  theme={null}
# ユーザースコープにインストール（デフォルト）
claude plugin install formatter@my-marketplace

# プロジェクトスコープにインストール（チームと共有）
claude plugin install formatter@my-marketplace --scope project

# ローカルスコープにインストール（gitignored）
claude plugin install formatter@my-marketplace --scope local
```

### plugin uninstall

インストール済みプラグインを削除します。

```bash  theme={null}
claude plugin uninstall <plugin> [options]
```

**引数:**

* `<plugin>`: プラグイン名または`plugin-name@marketplace-name`

**オプション:**

| オプション                 | 説明                                          | デフォルト  |
| :-------------------- | :------------------------------------------ | :----- |
| `-s, --scope <scope>` | アンインストール対象スコープ: `user`、`project`、または`local` | `user` |
| `-h, --help`          | コマンドのヘルプを表示                                 |        |

**エイリアス:** `remove`、`rm`

### plugin enable

無効なプラグインを有効にします。

```bash  theme={null}
claude plugin enable <plugin> [options]
```

**引数:**

* `<plugin>`: プラグイン名または`plugin-name@marketplace-name`

**オプション:**

| オプション                 | 説明                                     | デフォルト  |
| :-------------------- | :------------------------------------- | :----- |
| `-s, --scope <scope>` | 有効にするスコープ: `user`、`project`、または`local` | `user` |
| `-h, --help`          | コマンドのヘルプを表示                            |        |

### plugin disable

プラグインをアンインストールせずに無効にします。

```bash  theme={null}
claude plugin disable <plugin> [options]
```

**引数:**

* `<plugin>`: プラグイン名または`plugin-name@marketplace-name`

**オプション:**

| オプション                 | 説明                                     | デフォルト  |
| :-------------------- | :------------------------------------- | :----- |
| `-s, --scope <scope>` | 無効にするスコープ: `user`、`project`、または`local` | `user` |
| `-h, --help`          | コマンドのヘルプを表示                            |        |

### plugin update

プラグインを最新バージョンに更新します。

```bash  theme={null}
claude plugin update <plugin> [options]
```

**引数:**

* `<plugin>`: プラグイン名または`plugin-name@marketplace-name`

**オプション:**

| オプション                 | 説明                                              | デフォルト  |
| :-------------------- | :---------------------------------------------- | :----- |
| `-s, --scope <scope>` | 更新するスコープ: `user`、`project`、`local`、または`managed` | `user` |
| `-h, --help`          | コマンドのヘルプを表示                                     |        |

***

## デバッグと開発ツール

### デバッグコマンド

`claude --debug`を使用してプラグイン読み込みの詳細を確認します:

```bash  theme={null}
claude --debug
```

これは以下を表示します:

* どのプラグインが読み込まれているか
* プラグインマニフェストのエラー
* コマンド、エージェント、フック登録
* MCPサーバー初期化

### 一般的な問題

| 問題                                  | 原因                         | 解決策                                                                   |
| :---------------------------------- | :------------------------- | :-------------------------------------------------------------------- |
| プラグインが読み込まれない                       | 無効な`plugin.json`           | `claude plugin validate`または`/plugin validate`でJSON構文を検証               |
| コマンドが表示されない                         | ディレクトリ構造が間違っている            | `commands/`がルートにあることを確認、`.claude-plugin/`内ではない                        |
| フックが発火しない                           | スクリプトが実行可能でない              | `chmod +x script.sh`を実行                                               |
| MCPサーバーが失敗                          | `${CLAUDE_PLUGIN_ROOT}`がない | すべてのプラグインパスに変数を使用                                                     |
| パスエラー                               | 絶対パスが使用されている               | すべてのパスは相対的で`./`で始まる必要があります                                            |
| LSP `Executable not found in $PATH` | 言語サーバーがインストールされていない        | バイナリをインストール（例：`npm install -g typescript-language-server typescript`） |

### エラーメッセージの例

**マニフェスト検証エラー**:

* `Invalid JSON syntax: Unexpected token } in JSON at position 142`: コンマの欠落、余分なコンマ、または引用符なしの文字列を確認
* `Plugin has an invalid manifest file at .claude-plugin/plugin.json. Validation errors: name: Required`: 必須フィールドが欠落している
* `Plugin has a corrupt manifest file at .claude-plugin/plugin.json. JSON parse error: ...`: JSON構文エラー

**プラグイン読み込みエラー**:

* `Warning: No commands found in plugin my-plugin custom directory: ./cmds. Expected .md files or SKILL.md in subdirectories.`: コマンドパスは存在しますが、有効なコマンドファイルが含まれていない
* `Plugin directory not found at path: ./plugins/my-plugin. Check that the marketplace entry has the correct path.`: マーケットプレイスの`source`パスが存在しないディレクトリを指している
* `Plugin my-plugin has conflicting manifests: both plugin.json and marketplace entry specify components.`: 重複するコンポーネント定義を削除するか、マーケットプレイスエントリで`strict: true`を設定

### フックトラブルシューティング

**フックスクリプトが実行されない**:

1. スクリプトが実行可能であることを確認: `chmod +x ./scripts/your-script.sh`
2. シバンラインを確認: 最初の行は`#!/bin/bash`または`#!/usr/bin/env bash`である必要があります
3. パスが`${CLAUDE_PLUGIN_ROOT}`を使用していることを確認: `"command": "${CLAUDE_PLUGIN_ROOT}/scripts/your-script.sh"`
4. スクリプトを手動でテスト: `./scripts/your-script.sh`

**フックが予期されたイベントでトリガーされない**:

1. イベント名が正しいことを確認（大文字小文字を区別）: `PostToolUse`、`postToolUse`ではない
2. マッチャーパターンがツールと一致することを確認: ファイル操作用の`"matcher": "Write|Edit"`
3. フックタイプが有効であることを確認: `command`、`prompt`、または`agent`

### MCPサーバートラブルシューティング

**サーバーが起動しない**:

1. コマンドが存在し、実行可能であることを確認
2. すべてのパスが`${CLAUDE_PLUGIN_ROOT}`変数を使用していることを確認
3. MCPサーバーログを確認: `claude --debug`は初期化エラーを表示します
4. Claude Code外でサーバーを手動でテスト

**サーバーツールが表示されない**:

1. サーバーが`.mcp.json`またはplugin.jsonで正しく設定されていることを確認
2. サーバーがMCPプロトコルを正しく実装していることを確認
3. デバッグ出力で接続タイムアウトを確認

### ディレクトリ構造の間違い

**症状**: プラグインが読み込まれますが、コンポーネント（コマンド、エージェント、フック）が欠落しています。

**正しい構造**: コンポーネントはプラグインルートにある必要があり、`.claude-plugin/`内ではありません。`plugin.json`のみが`.claude-plugin/`に属します。

```
my-plugin/
├── .claude-plugin/
│   └── plugin.json      ← マニフェストのみここ
├── commands/            ← ルートレベル
├── agents/              ← ルートレベル
└── hooks/               ← ルートレベル
```

コンポーネントが`.claude-plugin/`内にある場合は、プラグインルートに移動してください。

**デバッグチェックリスト**:

1. `claude --debug`を実行して「loading plugin」メッセージを探します
2. 各コンポーネントディレクトリがデバッグ出力にリストされていることを確認
3. プラグインファイルを読み取ることができるファイルパーミッションを確認

***

## 配布とバージョン管理リファレンス

### バージョン管理

プラグインリリースのセマンティックバージョニングに従います:

```json  theme={null}
{
  "name": "my-plugin",
  "version": "2.1.0"
}
```

**バージョン形式**: `MAJOR.MINOR.PATCH`

* **MAJOR**: 破壊的な変更（互換性のないAPI変更）
* **MINOR**: 新機能（後方互換性のある追加）
* **PATCH**: バグ修正（後方互換性のある修正）

**ベストプラクティス**:

* 最初の安定版リリースは`1.0.0`から開始
* 変更を配布する前に`plugin.json`のバージョンを更新
* `CHANGELOG.md`ファイルで変更を文書化
* テスト用に`2.0.0-beta.1`のようなプレリリースバージョンを使用

***

## 関連項目

* [プラグイン](/ja/plugins) - チュートリアルと実践的な使用法
* [プラグインマーケットプレイス](/ja/plugin-marketplaces) - マーケットプレイスの作成と管理
* [スラッシュコマンド](/ja/slash-commands) - コマンド開発の詳細
* [サブエージェント](/ja/sub-agents) - エージェント設定と機能
* [エージェントスキル](/ja/skills) - Claudeの機能を拡張
* [フック](/ja/hooks) - イベント処理と自動化
* [MCP](/ja/mcp) - 外部ツール統合
* [設定](/ja/settings) - プラグインの設定オプション


---

> To find navigation and other pages in this documentation, fetch the llms.txt file at: https://code.claude.com/docs/llms.txt