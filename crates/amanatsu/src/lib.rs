//! # amanatsu
//!
//! OpenAPI 3.x・Swagger 2.0・JSON Schema の文書（YAML か JSON）を、**JavaScript なしの静的 HTML** にする。
//! 静的サイトのビルド時に API 仕様をページへ埋め込む用途向け。
//!
//! - 入力は [shikuwasa](https://crates.io/crates/shikuwasa) で `serde_json::Value` に読み、値をたどって HTML を
//!   組み立てる（構造体に落とさないので、知らないフィールドがあっても止まらない）
//! - **失敗しても `Err` を返さない** — パースの失敗・対応していない形式はエラーボックスの HTML を返す。
//!   警告は [`Rendered::warnings`] に入れて返す（ログに出すかは呼び出し側が決める）
//! - 出力は決定的（仕様の記述順を保つ）。埋め込む文字列はすべて [`escape_html`] を通す
//! - `$ref` は文書の中（`#/...`）と別ファイル（`path#/pointer`）を解決する。ファイルの読み込みは
//!   [`SpecFiles`] を通してだけ行う（この crate 自身は I/O をしない）。循環・未解決・リモートの参照は
//!   注記を出すだけにする
//!
//! 出力の文言は日本語（「パラメータ」「必須」など）。見た目は CSS のクラス（`api-spec`・`api-op` など）に
//! 任せている。クラスの一覧は README を参照。
//!
//! ```
//! use amanatsu::{NoFiles, SpecKind};
//!
//! let spec = "openapi: 3.0.3\ninfo: {title: 例, version: '1'}\npaths:\n  /items:\n    get:\n      responses:\n        '200': {description: 成功}\n";
//! let out = amanatsu::render(SpecKind::OpenApi, spec, None, &NoFiles);
//! assert!(out.html.contains("api-spec"));
//! assert!(out.warnings.is_empty());
//!
//! let broken = amanatsu::render(SpecKind::OpenApi, "openapi: [", None, &NoFiles);
//! assert!(broken.html.contains("API 仕様のレンダリングに失敗しました"));
//! assert_eq!(broken.warnings.len(), 1);
//! ```

#![forbid(unsafe_code)]

mod render;

/// 描画する文書の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecKind {
    /// OpenAPI 3.x / Swagger 2.0 の文書全体
    OpenApi,
    /// 1 つの JSON Schema
    JsonSchema,
}

impl SpecKind {
    /// すべての種類
    pub const ALL: [SpecKind; 2] = [SpecKind::OpenApi, SpecKind::JsonSchema];
}

/// 仕様ファイルの読み込み口。別ファイルへの `$ref`（`schemas/common.yaml#/User`）を解決するときに呼ぶ。
///
/// `path` は `/` 区切りに正規化した相対パス（`..` で基準の外へ出る参照は、呼ぶ前に注記にしている）。
/// 基準は、仕様ファイルの中の参照ならそのファイルのディレクトリ、`render` の `origin` が `None` の文書の
/// 中の参照なら呼び出し側が決めた基準（たとえばプロジェクトのルート）。基準の外を読ませない検査や、
/// 読んだことの記録は実装側で行う。`Err` の文字列は利用者にそのまま見せる
pub trait SpecFiles {
    fn read(&self, path: &str) -> Result<String, String>;
}

/// ファイルを読まない [`SpecFiles`]（別ファイルへの `$ref` はすべて読めないという注記になる）
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFiles;

impl SpecFiles for NoFiles {
    fn read(&self, _path: &str) -> Result<String, String> {
        Err("このコンテキストではファイル参照が使えません".to_string())
    }
}

/// [`render`] の結果
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Rendered {
    /// 埋め込む HTML（失敗したときはエラーボックス）
    pub html: String,
    /// 描画中に見つけた問題（パースの失敗・対応していない形式・参照先のファイルを読めない等）
    pub warnings: Vec<Warning>,
}

/// 描画中の警告
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Warning {
    /// 問題のあった参照先のファイル（[`SpecFiles::read`] に渡したパス）。文書そのものの問題なら `None`
    pub file: Option<String>,
    /// 日本語の説明
    pub message: String,
}

/// [`check`] が見つけた問題 1 件
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Issue {
    /// 説明（描画のエラーボックス・注記と同じ文言）
    pub message: String,
    /// 致命的か（描画すると文書全体がエラーボックスになる）。`false` は注記で済むもの
    pub fatal: bool,
}

/// 仕様のテキスト（YAML / JSON）を静的 HTML にする。
///
/// `origin` は文書のパス（`files` に渡すパスと同じ数え方）で、文書の中の相対の `$ref` の基準になる。
/// 文書をファイルから読んでいなければ `None`（相対の `$ref` は `files` の基準から数える）
pub fn render(
    kind: SpecKind,
    source: &str,
    origin: Option<&str>,
    files: &dyn SpecFiles,
) -> Rendered {
    render::render(kind, source, origin, files)
}

/// 仕様のテキストを検査する（HTML は作らない）。空なら問題なし。[`render`] がエラーボックス・注記に
/// するものと同じ分類・同じ文言を返す
pub fn check(
    kind: SpecKind,
    source: &str,
    origin: Option<&str>,
    files: &dyn SpecFiles,
) -> Vec<Issue> {
    render::check(kind, source, origin, files)
}

/// エラーボックスの HTML（GitHub の alert と同じ `markdown-alert markdown-alert-caution` の構造）。
/// `message` を説明の行に、`source` をエスケープして `<pre>` で添える
pub fn error_box(message: &str, source: &str) -> String {
    format!(
        "<div class=\"markdown-alert markdown-alert-caution\">\n\
         <p class=\"markdown-alert-title\">API 仕様のレンダリングに失敗しました</p>\n\
         <p>{}</p>\n\
         </div>\n\
         <pre><code>{}</code></pre>\n",
        escape_html(message),
        escape_html(source),
    )
}

/// HTML のテキストと属性値に埋める文字列のエスケープ（`&`・`<`・`>`・`"`）。この crate の出力と同じ規則
///
/// ```
/// assert_eq!(amanatsu::escape_html("<a href=\"x\">&</a>"), "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;");
/// ```
pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}
