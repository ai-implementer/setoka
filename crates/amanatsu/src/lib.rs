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
/// 中の参照なら呼び出し側が決めた基準（たとえばプロジェクトのルート）。`Err` の文字列は利用者にそのまま見せる。
///
/// **基準の外を読ませない検査は実装側で行う**。この crate のパスの正規化は字句の上の処理だけで、
/// シンボリックリンクの先が外かどうかは見ない（実装では `canonicalize` した結果が基準の下にあるかを確かめる。
/// README に例がある）。読んだことの記録（キャッシュの無効化など）も実装側で行う。
///
/// 1 回の描画・検査で呼ぶのは参照先のパスごとに 1 回。読み込みとパースに成功した参照先が 16 個に達したら、
/// それより後の参照先は読まずに注記にする（読めなかった参照先は数えないので、読めない参照先が多いと
/// その数だけ呼ぶ）
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
    /// 検査の上での重大度。`true` は直すべき書き間違い（文書のパースの失敗・対応していない版・参照先の
    /// ファイルを読めない・パースできない）、`false` は設計上の制限（参照先のファイルの数の上限）。
    /// 描画の結果とは一致しない: 文書全体がエラーボックスになるのはパースの失敗と対応していない版だけで、
    /// 参照先のファイルの問題は `true` でも描画では注記になる
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

/// 仕様のテキストを検査する（HTML は作らない）。空なら問題なし。文言は [`render`] のエラーボックス・注記と同じ。
///
/// 検査するのは、文書のパース・OpenAPI の版（3.x か Swagger 2.0 か）・別ファイルへの `$ref` の読み込みと
/// パース・参照先のファイルの数の上限だけ。文書の中の参照（`#/...`）の指す先があるか・参照の循環・
/// リモートの参照・スキーマの中身は見ない（描画ではこれらも注記になる）
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

/// HTML のテキストと、**二重引用符（`"`）で囲んだ属性値**に埋める文字列のエスケープ（`&`・`<`・`>`・`"`）。
/// この crate の出力と同じ規則。アポストロフィ（`'`）は変換しないので、単一引用符で囲んだ属性値や、
/// 引用符で囲まない属性値には使えない
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
