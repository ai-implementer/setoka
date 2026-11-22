# shikuwasa 🍋

YAML を**位置付きの木として安全に読む** Rust ライブラリ。Markdown の frontmatter や
OpenAPI / JSON Schema のように、人が書いた YAML を読んで、書き間違いを行と列付きで返したい場面向け。
名前は柑橘のシークヮーサーから。

パーサは [saphyr](https://crates.io/crates/saphyr)（純 Rust・YAML 1.2）で、その上に次の 4 つを足す。

- **別名の展開に上限を掛ける** — 入れ子の別名（`*x`）で指数的に膨らむ文書（いわゆる billion laughs）を
  展開 20 万ノードで止める（`MAX_ALIAS_NODES`）
- **同名のキーをエラーにする** — 引用符やタグだけが違う同名のキー（`draft` と `"draft"`）も重複とみる。
  後の値が黙って勝たない
- **スカラーの型を決める** — YAML 1.2 の core schema で解釈し、境界は serde_yaml_ng と同じにしてある。
  先頭 0 の 10 進数（`010`）・`_` 区切り（`1_000`）・YAML 1.1 の真偽値（`yes` / `on`）は文字列。
  明示のコアタグ（`!!bool "true"`）は引用符より先に見て、型に合わない値（`!!int "x"`）はエラー
- **位置はバイト** — ノードとエラーの位置は入力の中のバイト位置（saphyr の位置は文字数）。
  `line_col` で 1 始まりの行とバイト基準の列にできる

ほかに、`---` で区切った 2 つ目の文書はエラーにする（1 つの文書だけを読む）。

**エラーの文言は日本語**（書いた人にそのまま見せるため。例:「キーが重複しています（同じキーは 1 回だけ
書けます。引用符の有無は区別しません）」）。

開発は [setoka](https://github.com/ai-implementer/setoka) の monorepo で行っている
（setoka の frontmatter と API 仕様の読み込みが使う）が、`setoka-*` には依存しない独立のライブラリ。

## インストール

```bash
cargo add shikuwasa
# YAML を serde_json::Value へ写す to_json も使うなら
cargo add shikuwasa --features json
```

## 使い方

```rust
use shikuwasa::{Kind, ScalarValue};

let text = "title: 設計書\norder: 2\ndraft: yes\n";
let root = shikuwasa::parse(text).unwrap().expect("文書がある");
let Kind::Mapping(entries) = &root.kind else { unreachable!() };
for (key, value) in entries {
    let (line, col) = shikuwasa::line_col(text, value.offset);
    let scalar = value.as_scalar().expect("スカラー");
    println!("{}（{line} 行 {col} 列）= {:?}", key.as_scalar().unwrap().text, scalar.value);
}
// draft: yes は YAML 1.2 では文字列
assert_eq!(entries[2].1.as_scalar().unwrap().value, ScalarValue::Str);

let e = shikuwasa::parse("title: a\ntitle: b\n").unwrap_err();
assert_eq!((e.line, e.col), (2, 1));
println!("{e}"); // キーが重複しています（…）（2 行 1 列）
```

- `parse(text)` は文書が無ければ（空・コメントだけ）`Ok(None)`
- `Node` は `offset`（バイト位置）・`tag`・`kind`（`Scalar` / `Sequence` / `Mapping`）を持つ。
  マッピングは書いた順のキーと値の組
- `Scalar` は原文（`text`）・書き方（`style`。引用符・ブロック）・型の解釈（`value`）を持つ。
  `200:` のように書いたキーも原文は `"200"`
- `resolve_plain(text)` は引用符もタグも無いスカラーとして解釈した結果を返す（「引用符を外せば整数」の
  ような案内を作るときに使う）

feature `json` の `to_json(text)` は YAML（JSON を含む）を `serde_json::Value` へ写す。キーはスカラーの
原文・マッピングの記述順を保つ（serde_json の `preserve_order`）・有限でない小数（`.inf`・`.nan`）は
null・キーが配列やマッピングならエラー。

## 制限

- 1 つの文書だけを読む（複数の文書はエラー）
- 集合（配列・マッピング）に付けたコアタグ（`!!map`・`!!set` など）は saphyr が残さないので、
  `Node::tag` は `None` になる
- 文字列以外の入力（ファイル・バイト列）は読まない。読み込みは呼び出し側で行う

## ライセンス

MIT または Apache-2.0
