# hebesu 🍋

日本語の文章の**表記ゆれ**を見つける Rust ライブラリ。設計書やドキュメントの校正・lint 向け。
名前は柑橘のへべすから。

| ルール | 見つけるもの |
| --- | --- |
| `fullwidth-alphanumeric` | 全角英数字（`ＡＢＣ１２３`）。半角を提案する |
| `halfwidth-kana` | 半角カナ（`ｻｰﾊﾞ`）。全角を提案する（濁点・半濁点は前の文字と合成する） |
| `term-variant` | 用語辞書のゆれ（`サーバ` → `サーバー`）。正表記の一部としての出現は除く |
| `katakana-choon` | 長音符のゆれの混在（`サーバ` と `サーバー`）。複数の文章を集計して、少数派の出現を報告する |

- **依存ゼロ・I/O なし** — 文字列を受け取って、見つけたゆれ（バイトの範囲・見つけた表記・置き換えの候補・
  日本語の説明）を返すだけ。Markdown などの構造は読まないので、本文のテキストを取り出すのは使う側
- **自動修正に使える** — 候補（`suggestion`）で範囲を置き換えれば直る。長音符のゆれでいちばん多い表記が
  複数あるときは、正しい表記を決められないので候補は無し
- **ルールの名前** は [setoka](https://github.com/ai-implementer/setoka) の診断の名前と同じ。説明の文言は日本語

開発は setoka の monorepo で行っている（setoka の `setoka lint` の表記ゆれのルールがこれを使う）が、`setoka-*` には
依存しない独立のライブラリ。

## インストール

```bash
cargo add hebesu
```

## 使い方

```rust
use std::collections::BTreeMap;

let text = "Ｒｕｓｔでｻｰﾊﾞを書く";
for f in hebesu::fullwidth_alphanumeric(text).into_iter().chain(hebesu::halfwidth_kana(text)) {
    // f.range は text の中のバイトの範囲。f.suggestion で置き換えれば直る
    println!("{}: {}（{:?}）", f.rule.name(), f.message, f.suggestion);
}

// 用語辞書（正表記 → ゆれ表記の一覧）
let terms = BTreeMap::from([("サーバー".to_string(), vec!["サーバ".to_string()])]);
let found = hebesu::term_variants("サーバーとサーバ", &terms);
assert_eq!(found[0].suggestion.as_deref(), Some("サーバー"));

// 長音符のゆれは文章の集まりで多数派を決める。位置の印（ここではファイル名）は使う側が決める型
let mut tally = hebesu::ChoonTally::new();
tally.add("ユーザーを追加する", "a.md");
tally.add("ユーザーを消す", "b.md");
tally.add("ユーザを探す", "c.md");
for (file, f) in tally.finish() {
    println!("{file}: {}", f.message); // c.md: 長音符ゆれ「ユーザ」が混在しています（…）
}
```

## 長音符のゆれの決めごと

- 長音符（ー）を除いた形が同じカタカナ語を、同じ語とみなす（`サーバ` と `サーバー`）
- 表記が 2 種類以上あれば、単独の多数派以外の出現を報告し、多数派を候補にする。いちばん多い表記が複数あれば、
  どの表記も報告して候補は無し
- 長音符を除いて 2 文字未満の語（`ルー`）と、いちばん短い表記が 3 文字未満の語（`カド` と `カード` のような
  別の語の偶然の一致）は対象外

## ライセンス

MIT または Apache-2.0
