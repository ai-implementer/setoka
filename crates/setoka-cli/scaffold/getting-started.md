---
title: はじめに
order: 1
description: setoka で書くときに知っておくこと
# ページを移動しても旧 URL を生かす（/guide/intro/ にリダイレクト HTML が出ます）
aliases: ["guide/intro/"]
---

# はじめに

## 書く・確かめる・出力する

```bash
setoka dev      # 開発サーバ（保存すると自動で作り直し、ブラウザを再読み込みする）
setoka check    # リンク切れ・書き方の誤り・整形差分を調べる
setoka build    # dist/ に静的サイトを出力する
setoka preview  # 出力した dist/ を http://127.0.0.1:5173/ で確かめる
```

執筆中は `setoka dev` の 1 コマンドで足ります（ライブリロード）。自動の再読み込みが動かない
環境では、`setoka build --watch` を使ってください。`dist/` の HTML はファイルとして直接開くと
検索などが動かないので、出力を確かめるときは `setoka preview` を使います。

## ページの並びと表示名

`content/` のフォルダの階層が、そのままサイドバーの階層になります。各ページの先頭の
frontmatter で、表示名と並び順を決めます。

```yaml
---
title: ページタイトル # サイドバーの表示名（無ければ最初の見出し → ファイル名）
order: 1 # 並び順（無ければファイル名の順で後ろに並ぶ）
description: 説明 # 検索結果や共有カードに出る説明
draft: true # 下書き（通常のビルドから外す。setoka dev --drafts で確かめられる）
aliases: ["old-url/"] # ページを移したときの旧 URL（転送のページを出す）
---
```

フォルダの表示名と並び順は、その中の `index.md` の `title` と `order` で決まります
（このサイトでは `content/guide/index.md`）。

h2・h3 の見出しは右側の「目次」に載り、見出しの右の `#` がその見出しへのリンクです。
ヘッダー右上の ◐ ボタンでダークモードに切り替えられます（選んでいなければ OS の設定に従います）。

## リンクと画像

ほかのページへは、原稿の `.md` へのパスでリンクします。ビルドすると正しい URL に変わります。

```markdown
[記法の例](examples.md)
[ホーム](../index.md)
[記法の例のコードブロック](examples.md#コードブロック)
```

画像の置き方は 2 通りです。

- `public/` の下に置くと、そのまま `dist/` にコピーされます。`/images/setoka-logo.svg` のように
  サイトの絶対パスで参照します
- ページと同じフォルダに置くと、`![図](diagram.png)` のように相対パスで参照できます

![setoka ロゴ](/images/setoka-logo.svg)

リンク切れ・画像切れは `setoka check` が見つけます。

## 品質チェック

```bash
setoka fmt     # 原稿を決まった形に整形する
setoka lint    # 表記ゆれ（全角英数字・半角カナ・長音符のゆれ・用語辞書）を調べる
setoka check   # lint に加えて、リンク切れ・取り込みの誤り・整形差分を調べる
```

終了コード（0 = 問題なし / 1 = 違反あり / 2 = 実行エラー）で CI に組み込めます。
GitHub Actions なら `setoka check --format github` で、PR の該当行に注釈が付きます。

## 公開する

同梱の `.github/workflows/deploy.yml` は、`main` への push で GitHub Pages に公開します。
リポジトリの Settings の Pages で、Source を「GitHub Actions」にしてから push してください。
ワークフローが使う setoka の版は `SETOKA_VERSION` で固定されています。

GitHub Pages はリポジトリが非公開でも公開されます。社内だけに見せるときや、GitHub Pages 以外に
置くときは、setoka のドキュメントの[社内で公開する](https://ai.implementer.net/setoka/guide/internal/)と
[配信とデプロイ](https://ai.implementer.net/setoka/guide/deploy/)を参照してください。

## 次に読む

- [記法の例](examples.md) — コード・図・数式・注意書き・API 仕様などの見本
- [setoka のドキュメント](https://ai.implementer.net/setoka/) — 設定・コマンド・記法の詳しい説明
