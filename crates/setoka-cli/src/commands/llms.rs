//! `setoka llms [--full]`: llms.txt / llms-full.txt をその場で生成して標準出力へ。
//! dist/ 不要のドライラン兼エクスポート（`setoka llms --full | pbcopy` で LLM に直接渡せる）。
//! 明示実行なので `llms.enabled` に関わらず生成する（`setoka search` と同じ思想）

pub fn run(cx: &crate::cx::Cx, full: bool) -> anyhow::Result<()> {
    let rc = super::load_project(cx)?;

    let site = setoka_core::build_site_model(
        &rc.content_dir,
        &rc.config.input.ignore,
        &setoka_render::markdown_options(&rc.config),
    )?;

    let text = if full {
        setoka_render::generate_llms_full_txt(&rc, &site, None)?
    } else {
        setoka_render::generate_llms_txt(&rc, &site)?
    };
    // 全文を 1 回で書く（`| head` で下流が閉じても panic しない。out.rs 参照）
    crate::out::str(&text);
    Ok(())
}
