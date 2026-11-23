//! OpenAPI / JSON Schema のビルド時レンダリング（SSR）の setoka 側。
//!
//! 描画と検査の本体は crate amanatsu（v0.22 Phase 103 で apispec から切り出した）。ここに残すのは
//! setoka の決めごとだけ:
//! - フェンスの言語名（` ```openapi ` / ` ```jsonschema `）と [`SpecKind`] の対応（`setoka_core::SPEC_LANGS`
//!   との一致は speccheck のテストが縛る）
//! - 描画の警告をページ名付きでログに出す（amanatsu はログを出さず、警告を結果で返す）
//!
//! ファイルの読み込み口（[`SpecFiles`] の実装）は、描画が highlight.rs、検査が speccheck.rs に持つ

pub(crate) use amanatsu::{SpecFiles, SpecKind, error_box};

/// [`SpecKind`] のフェンス言語名
pub(crate) fn lang(kind: SpecKind) -> &'static str {
    match kind {
        SpecKind::OpenApi => "openapi",
        SpecKind::JsonSchema => "jsonschema",
    }
}

/// フェンス言語 → 描画種別。描画のディスパッチと `setoka check` の両方が
/// これを使う（言語名の直書きを増やさない）
pub(crate) fn spec_kind(lang_name: &str) -> Option<SpecKind> {
    SpecKind::ALL.into_iter().find(|&k| lang(k) == lang_name)
}

/// 仕様テキストを静的 HTML へ変換し、警告をログに出す（`page` は警告に付けるページ名）
pub(crate) fn render_spec(
    kind: SpecKind,
    source: &str,
    origin: Option<&str>,
    files: &dyn SpecFiles,
    page: &str,
) -> String {
    let rendered = amanatsu::render(kind, source, origin, files);
    for warning in &rendered.warnings {
        match &warning.file {
            Some(file) => {
                tracing::warn!(page = %page, file = %file, "{}", warning.message);
            }
            None => tracing::warn!(page = %page, "{}", warning.message),
        }
    }
    rendered.html
}
