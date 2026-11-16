---
title: mdBook との比較
order: 3
description: setoka と mdBook の違いと、どちらが合うかの目安
---

# mdBook との比較

setoka と同じく、Rust で作られた 1 つのバイナリで Markdown から静的なサイトを作るツールに
[mdBook](https://github.com/rust-lang/mdBook) があります。どちらを使うかを決める目安として、
違いをまとめます。

mdBook について書いたことは、mdBook v0.5.4（2026-07-06 のリリース）の公式のドキュメントと
ソースで、2026-11-16 に確かめた内容です。

## どちらが合うか

setoka が合うのは、次のような場合です。

- 日本語の文書を全文検索したい（分かち書きで語を区切る）
- 表記ゆれ・リンク切れ・整形の検査を、ほかのツールを足さずに CI に組み込みたい
- ページの並びを、ファイルとフォルダの置き方で決めたい（目次のファイルを手で保たない）
- 図（Mermaid）とコードのハイライトを、ビルド時に HTML にしたい

mdBook が合うのは、次のような場合です。

- Rust のコード例を、ページから Rust Playground で実行させたい・`mdbook test` でテストしたい
- 本の章立てのように、目次のファイルで順番と階層を細かく決めたい
- プリプロセッサやバックエンド（第三者のプラグインを含む）で機能を足したい
- テーマの文言が英語のサイトを作りたい（setoka のテーマの文言は日本語だけです）

## 機能の違い

| 項目 | setoka | mdBook v0.5.4 |
| --- | --- | --- |
| ページの並び | `content/` のフォルダの階層。並び順は frontmatter の `order` | `SUMMARY.md` に章の順番と階層を書く（[SUMMARY.md](https://rust-lang.github.io/mdBook/format/summary.html)） |
| 全文検索 | 日本語の分かち書き・誤字に寛容・フレーズ検索 | 空白と `-` で語を区切る（英語向け）。80 バイトを超える語は索引に入れないので、空白の無い日本語が 27 文字以上続くと、その部分は検索できない（[search.rs](https://github.com/rust-lang/mdBook/blob/v0.5.4/crates/mdbook-html/src/html_handlebars/search.rs)） |
| コードのハイライト | ビルド時に HTML にする | Highlight.js（[Syntax highlighting](https://rust-lang.github.io/mdBook/format/theme/syntax-highlighting.html)） |
| 図（Mermaid） | 同梱の mermaid.js。設定で 10 図種をビルド時に SVG にする | 本体には無い。第三者のプリプロセッサ mdbook-mermaid で足す（[Third party plugins](https://github.com/rust-lang/mdBook/wiki/Third-party-plugins)） |
| 数式 | 同梱の KaTeX（ブラウザで描画） | MathJax（`mathjax-support`。既定は無効）（[Renderers](https://rust-lang.github.io/mdBook/format/configuration/renderers.html)） |
| 外部ファイルの取り込み | コードブロックの `file=` と `lines=`・Markdown の断片 | `{{#include}}`（行の範囲・アンカー）・`{{#rustdoc_include}}`（[mdBook 固有の記法](https://rust-lang.github.io/mdBook/format/mdbook.html)） |
| 注意書き・定義リスト | あり | あり（[Markdown](https://rust-lang.github.io/mdBook/format/markdown.html)） |
| 品質の検査 | `fmt`・`lint`（表記ゆれ）・`check`（リンク切れ・取り込み・整形）を内蔵 | 内蔵のコマンドは `init`・`build`・`watch`・`serve`・`test`・`clean`・`completions`（[CLI](https://rust-lang.github.io/mdBook/cli/index.html)）。リンク検査は第三者の mdbook-linkcheck |
| コード例の実行とテスト | 無い | Rust Playground で実行・`mdbook test` |
| 多言語のサイト | 無い | 本体には無い。第三者の mdbook-i18n-helpers など |
| 機能の追加 | プラグインの仕組みは無い | プリプロセッサとバックエンド |
| 最終更新日 | git の履歴から出せる（`git.last_updated`） | 組み込みの設定は無い |
| 編集のリンク | `git.edit_url` | `edit-url-template` |

書き方の違い（目次の作り方・リンク・注意書きなど）は [他のツールとの違い](migrate.md) にまとめています。
