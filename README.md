# LustUI

LustUI は、Ollamaと連携してローカル環境で自動開発・コマンド実行を行うGUI/CLIエージェントツールです。

## インストール方法

PowerShellを開き、以下のコマンドを1行実行するだけでインストールおよび環境構築が完了します。

```powershell
irm https://raw.githubusercontent.com/loprabbit-tech/Lustui/main/install.ps1 | iex
```

> **Note**: 管理者権限は不要です。ユーザー環境（`%LOCALAPPDATA%\Lustui`）に自動配置され、PATHの追加と `.lustuiprj` ファイルの関連付けが行われます。

---

## 使い方

### 1. 新規プロジェクトの作成

ターミナルで以下のコマンドを実行し、新規プロジェクトを作成します。

```cmd
lustui new my_project
```

実行すると、作業ディレクトリ `my_project.lustuiprj_dir` とプロジェクト定義ファイル `my_project.lustuiprj` が生成されます。

### 2. アプリケーションの起動

生成された `.lustuiprj` ファイルを指定して起動します。

```cmd
lustui my_project.lustuiprj
```

または、エクスプローラー上で **`my_project.lustuiprj` をダブルクリック** して起動することも可能です。

起動するとWebブラウザが自動的に開き、UI画面からローカルLLMへの指示およびツールの自動実行が行えます。

---

## 前提条件

* **Windows 10 / 11**
* **[Ollama](https://ollama.com/)**（ローカルLLMを実行するために必要です）
