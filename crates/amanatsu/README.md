# amanatsu 🍊

OpenAPI 3.x・Swagger 2.0・JSON Schema の文書（YAML か JSON）を、**JavaScript なしの静的 HTML** にする
Rust ライブラリ。静的サイトジェネレータが、ビルド時に API 仕様をページへ埋め込む用途向け。
名前は柑橘の甘夏から。

- **JS も外部の資産も要らない** — 出力は `<details>`・`<table>` などの素の HTML で、見た目は CSS のクラスに任せる
- **失敗してもビルドを止めない** — パースの失敗・対応していない形式は `Err` ではなくエラーボックスの HTML を
  返す。警告は結果の `warnings` に入れて返す（ログに出すかは呼び出し側が決める。この crate は何も出力しない）
- **`$ref` を解決する** — 文書の中（`#/components/schemas/User`）と別ファイル（`schemas/common.yaml#/User`）。
  ファイルの読み込みは `SpecFiles` の trait を通してだけ行い、この crate 自身は I/O をしない。循環・未解決・
  リモートの参照は注記を出すだけにする（参照するファイルは 1 文書から 16 個まで）
- **出力は決定的** — 仕様の記述順を保つ。埋め込む文字列はすべてエスケープする
- **検査だけもできる** — `check` は HTML を作らずに、描画がエラーボックス・注記にする問題を同じ文言で返す
  （CI で壊れた仕様を見つける用途）

YAML の読み込みは [shikuwasa](https://crates.io/crates/shikuwasa)（同名のキー・別名の展開の上限・YAML 1.2 の
型の解釈）。**出力の文言は日本語**（「パラメータ」「必須」「レスポンス」など）。

開発は [setoka](https://github.com/ai-implementer/setoka) の monorepo で行っている（setoka の ` ```openapi ` /
` ```jsonschema ` のコードブロックがこれで描画される）が、`setoka-*` には依存しない独立のライブラリ。

## インストール

```bash
cargo add amanatsu
```

## 使い方

```rust
use amanatsu::{NoFiles, SpecFiles, SpecKind};

let spec = std::fs::read_to_string("openapi.yaml")?;
let out = amanatsu::render(SpecKind::OpenApi, &spec, Some("openapi.yaml"), &DirFiles);
for w in &out.warnings {
    eprintln!("警告: {}{}", w.message, w.file.as_deref().map(|f| format!("（{f}）")).unwrap_or_default());
}
println!("{}", out.html);

// 別ファイルの $ref を読む口（基準のディレクトリの外を読ませない検査は実装側で行う）
struct DirFiles;
impl SpecFiles for DirFiles {
    fn read(&self, path: &str) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| format!("{path} を読めません: {e}"))
    }
}
```

- `render(kind, source, origin, files)` — `SpecKind::OpenApi`（OpenAPI 3.x / Swagger 2.0）か
  `SpecKind::JsonSchema`（1 つのスキーマ）。`origin` は文書のパスで、文書の中の相対の `$ref` の基準になる
  （ファイルから読んでいなければ `None`）。別ファイルを読まないなら `&NoFiles`
- `check(kind, source, origin, files)` — 問題の一覧（`message` と、文書全体がエラーボックスになるかの `fatal`）
- `error_box(message, source)` — 呼び出し側で読み込みに失敗したときに、同じ見た目のエラーボックスを作る
- `escape_html(text)` — 出力と同じ規則のエスケープ（`&`・`<`・`>`・`"`）

## 出力の HTML と CSS

全体は `<div class="api-spec">` で囲まれる。CSS は同梱していないので、使う側で用意する（setoka のテーマの
`theme.css` の `.api-spec` から始まる部分が例になる）。主なクラス:

| クラス | 中身 |
| --- | --- |
| `api-spec` | 全体 |
| `api-info`・`api-title`・`api-version`・`api-desc` | 文書の情報（タイトル・版・説明） |
| `api-paths`・`api-op`・`api-op-{method}` | パスと操作（`<details>`。`{method}` は `get` などの小文字） |
| `api-method`・`api-method-{method}` | メソッドのバッジ（`GET` など） |
| `api-summary`・`api-op-desc` | 操作の要約と説明 |
| `api-params`・`api-request-body`・`api-req`・`api-required-note` | パラメータの表・リクエストボディ・必須の印 |
| `api-responses`・`api-response`・`api-status`・`api-status-{cls}`・`api-status-desc` | レスポンス（`{cls}` は `2xx` などと `default`） |
| `api-media` | メディアタイプ（`application/json` など） |
| `api-schema`・`api-schema-table`・`api-schema-def`・`api-schemas` | スキーマの表・定義・スキーマの一覧 |
| `api-type`・`api-scalar`・`api-array-items`・`api-nested`・`api-nested-row` | 型・配列の要素・入れ子 |
| `api-combinator`・`api-combinator-label`・`api-combinator-list` | `oneOf` / `anyOf` / `allOf` |
| `api-ref`・`api-ref-name`・`api-anno`・`api-omitted`・`api-empty`・`api-section-label` | 参照の注記・省略・空・見出し |

エラーボックスは GitHub の alert と同じ構造（`<div class="markdown-alert markdown-alert-caution">` と
`<p class="markdown-alert-title">`）に、元のテキストの `<pre><code>` が続く。

## 制限

- OpenAPI は 3.x と Swagger 2.0 だけ（`openapi: 3.x.y` か `swagger: "2.0"` が要る）
- 入れ子のスキーマは 8 段まで描画し、それより深い所は省略の注記にする
- リモートの `$ref`（`https://…`）は解決しない（注記になる）

## ライセンス

MIT または Apache-2.0
