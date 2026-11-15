---
title: インストールと最初のサイト
order: 1
description: setoka のインストール・最初のサイトの作り方・既存のフォルダで始める方法
---

# インストールと最初のサイト

## インストール

[GitHub Releases](https://github.com/ai-implementer/setoka/releases/latest) から
お使いのプラットフォームのアーカイブ（macOS arm64 / x64・Linux x64・Windows x64）を
ダウンロードして展開し、`setoka` を PATH の通った場所へ置きます。
各リリースには検証用の `SHA256SUMS` も添付されています。

Rust ツールチェイン（1.99 以降）があれば、ソースからもインストールできます。
`vX.Y.Z` はリリースの版に置き換えてください（`--tag` を省くと、リリース前の
変更を含む main が入ります）:

```bash
cargo install --locked --git https://github.com/ai-implementer/setoka --tag vX.Y.Z setoka-cli
```

## プロジェクトを作る

```bash
setoka new my-docs
cd my-docs
```

`setoka new` は次の構成を生成します:

    my-docs/
    ├─ setoka.toml                     # 設定（TOML。すべてのキーが省略可能）
    ├─ content/                      # Markdown 原稿（ディレクトリ階層 = ナビ階層）
    │  ├─ index.md                   # トップページ
    │  └─ guide/
    │     ├─ index.md                # ディレクトリ「ガイド」の表示名と並び順
    │     └─ getting-started.md
    ├─ snippets/                     # 原稿から取り込むサンプル（コードと Markdown 断片）
    ├─ public/                       # 静的ファイル（そのまま配信される）
    │  └─ images/setoka-logo.svg
    ├─ theme/README.md               # テーマ上書きの説明（同じ相対パスのファイルを置くだけ）
    ├─ .gitignore                    # dist/ と .setoka/ を除外
    └─ .github/workflows/deploy.yml  # GitHub Pages 自動デプロイ

## 書く・確認する・出力する

```bash
setoka dev      # 開発サーバ（保存すると自動で再ビルドし、ブラウザを再読み込み）
setoka build    # dist/ に静的サイトを出力
setoka preview  # ビルド済みの dist/ を http://127.0.0.1:5173/ で確認
```

執筆中は `setoka dev` の 1 コマンドで十分です。プロジェクトの中のファイル
（原稿・テーマ・`file=` で取り込むファイルなど。`dist/` と隠しディレクトリを除く）を
監視して自動で再ビルドし、開いているブラウザを再読み込みします（保存から約 1 秒）。
`setoka dev` の自動再読み込みが動かない環境では、`setoka build --watch` を使ってください
（ブラウザが定期的に確認して再読み込みします）。

> [!NOTE]
> `setoka build` は 2 回目以降、変更したページだけを計算し直すので、ページ数が
> 増えても再ビルドは速いままです。変更が反映されないときは `setoka build --force` で
> 最初から作り直せます（仕組みは開発者向けの
> [インクリメンタルビルドの内部設計](../development/internals-build.md)）。

## dist はサーバで配信する

`dist/` のページのリンクと CSS・JavaScript の参照は、サイトのルート（`/`）から
始まります。そのため `dist/index.html` をダブルクリックしてファイルとして開くと
（アドレスが `file://` になる）、リンクやスタイルが崩れ、検索も動きません。
手元で確かめるときは `setoka preview` を使い、公開するときは Web サーバか
GitHub Pages などに置いてください（[配信とデプロイ](deploy.md)・[社内で公開する](internal.md)）。
サブパス（`https://example.com/docs/`）に置くときは `base_url` を指定します。

## 既存のフォルダで始める

Markdown がすでにあるフォルダでも始められます。`setoka new .` は空でない
フォルダでは止まるので、次の手順で `setoka.toml` を置きます。

1. フォルダの直下に `setoka.toml` を作ります。中身は空でも動きます
   （サイト名などはあとから[設定リファレンス](../reference/config.md)を見て足します）。
   `setoka new` で別の場所に作ったプロジェクトの `setoka.toml` をコピーしても構いません
2. 原稿を `content/` へ移すか、原稿のあるディレクトリを `input.dir` で指定します
   （下の例）
3. 原稿ディレクトリの直下に `index.md` を置きます。これがトップページになります
   （無いとサイトの `/` が 404 になり、`setoka build` が警告します。`README.md` はトップページに
   ならず `/README/` のページになるので、トップページにするなら `index.md` へ名前を変えます）
4. `setoka build` のあとに `setoka preview` で確認します
5. GitHub Pages へ公開するなら、[配信とデプロイ](deploy.md#ワークフローの全文)の
   ワークフローを `.github/workflows/deploy.yml` に置きます（`setoka new` で作ったプロジェクト
   には最初から入っています）

ほかのドキュメントツールで書いた原稿なら、[他のツールとの違い](migrate.md)で書き方の
違いを確かめてください。

原稿が `docs/` にある場合の `input.dir` の指定:

```toml
[input]
dir = "docs"
```

> [!WARNING]
> `input.dir` にプロジェクトルート（`.`）は指定できません。出力先の `dist/` が原稿の
> 中に入るため、設定エラーになります。フォルダの直下に Markdown を並べている場合は、
> サブディレクトリへ移してください。原稿ディレクトリが無い・中に公開する `.md` が無いと、
> ビルドは成功しますが「ビルドしたページが 0 件です」の警告が出ます。

## 品質チェックと公開

```bash
setoka fmt        # Markdown を決まった書き方へ整形（--check で CI 用の差分検出）
setoka lint --fix # 表記ゆれ（全角英数字・半角カナ・用語・長音符）を自動修正
setoka check      # lint + リンク切れ + fmt 差分の統合チェック（CI 用）
```

GitHub に push すると、同梱の `.github/workflows/deploy.yml` が
GitHub Pages へ自動デプロイします（リポジトリの Settings \> Pages \> Source を
「GitHub Actions」にするだけ。詳細は[配信とデプロイ](deploy.md)）。
