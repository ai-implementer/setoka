---
title: ホーム
order: 1
description: setoka のサンプルドキュメント
---

# ようこそ

これは `setoka new` が作ったサンプルのサイトです。ここから自分のドキュメントに書き換えていきます。

## 最初の 5 分

1. **開発サーバを起動する** — このフォルダで `setoka dev` を実行し、表示された URL をブラウザで
   開きます。原稿を保存すると、自動で作り直してブラウザを再読み込みします
2. **このページを書き換える** — このページの原稿は `content/index.md` です。見出しや文章を変えて
   保存してみてください
3. **ページを足す** — `content/` の下に `.md` を置くと、サイドバーに載ります。フォルダを作ると
   サイドバーの階層になります（例: `content/guide/getting-started.md`）
4. **確かめる** — `setoka check` で、リンク切れや書き方の誤りを調べます
5. **公開する** — GitHub に push すると、同梱の `.github/workflows/deploy.yml` が GitHub Pages に
   公開します（先にリポジトリの Settings の Pages で、Source を「GitHub Actions」にします）

## このサイトのページ

- [はじめに](guide/getting-started.md) — 書くときに知っておくこと（ページの並び・リンクと画像・
  品質チェック・公開）
- [記法の例](guide/examples.md) — コード・図・数式・注意書き・API 仕様などの見本。要らなくなったら
  消してかまいません
- [用語集](glossary.md) — `setoka.toml` の用語辞書から自動で作られるページ

使い方の詳しい説明は [setoka のドキュメント](https://ai.implementer.net/setoka/) にあります。
