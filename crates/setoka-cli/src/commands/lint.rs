//! `setoka lint [--fix]`: 文書規約の診断（見出し・frontmatter・表記ゆれ）。
//! リンク切れ・fmt 差分まで含めた統合チェックは `setoka check`
//!
//! `--fix` は表記ゆれ系の変換候補（[`setoka_core::Diagnostic::fix`]）をソースへ
//! 自動適用する。fmt と同じ規約: 冪等・差分のないファイルには書き込まない
//! （mtime を汚さない）。frontmatter は lint の対象外なので触れない。
//! 修正できない違反（見出し規約等）は従来どおり報告して終了コード 1

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::ExitCode;

use setoka_core::{Diagnostic, Page};

use super::diag;
use crate::out::outln;

/// fix の適用が別のゆれを生む連鎖に備えた再 lint の上限（通常は 1 周で収束）
const MAX_FIX_ROUNDS: usize = 10;

/// [`fix_until_stable`] の結果
#[derive(Debug, Default, PartialEq)]
struct FixOutcome {
    /// 適用した修正の件数（全ラウンドの合計）
    applied: usize,
    /// 書き換えた原稿（`Page::src`）
    files: BTreeSet<PathBuf>,
    /// 書き換えたラウンドの数（何も直らなかった最後のラウンドは数えない）
    rounds: usize,
}

/// 表記ゆれの修正を、直すものが無くなるまで（最大 `max_rounds` 回）繰り返す。
///
/// 1 ラウンドは「原稿を読む（`load`）→ 修正つきの診断を集める（`fixable`）→ ページごとに
/// 適用して書く（`write`）」。修正が別のゆれを生むことがあるので、書いた後に読み直す。
/// 上限に達したら、残りは直さずに返す（呼び出し側が残った違反として報告する）。
/// 合成ページ（用語集）は原稿が無いので書かない。適用が 0 件・結果が原稿と同じページも
/// 書かない（mtime を汚さない）。読み込み・診断・書き込みはテストで差し替える
fn fix_until_stable(
    max_rounds: usize,
    mut load: impl FnMut() -> anyhow::Result<Vec<Page>>,
    mut fixable: impl FnMut(&[Page]) -> anyhow::Result<Vec<Diagnostic>>,
    mut write: impl FnMut(&Page, &str) -> anyhow::Result<()>,
) -> anyhow::Result<FixOutcome> {
    let mut outcome = FixOutcome::default();
    for _ in 0..max_rounds {
        let pages = load()?;
        let diags = fixable(&pages)?;
        let mut applied_this_round = 0usize;
        for page in pages.iter().filter(|p| !p.is_generated()) {
            let page_diags: Vec<Diagnostic> = diags
                .iter()
                .filter(|d| d.rel == page.rel && d.fix.is_some())
                .cloned()
                .collect();
            if page_diags.is_empty() {
                continue;
            }
            let (fixed, applied) = setoka_core::apply_fixes(&page.source, &page_diags);
            // 適用 0 件（範囲交差で全スキップ等）なら書き込まない（mtime 温存）
            if applied == 0 || fixed == page.source {
                continue;
            }
            write(page, &fixed)?;
            applied_this_round += applied;
            outcome.files.insert(page.src.clone());
        }
        if applied_this_round == 0 {
            break; // 不動点（fix 対象なし or 全て適用不能）
        }
        outcome.applied += applied_this_round;
        outcome.rounds += 1;
    }
    Ok(outcome)
}

pub fn run(cx: &crate::cx::Cx, fix: bool, format: diag::Format) -> anyhow::Result<ExitCode> {
    let rc = super::load_project(cx)?;
    let root = &rc.root;
    let opts = setoka_render::markdown_options(&rc.config);
    // lint は check だけのルール（リンク・外部リンク・URL の衝突等）を評価しない（その抑制を unused にしない）
    let lint_opts = diag::lint_options(&rc, diag::Evaluates::Lint);
    let collect = |pages: &[Page]| -> anyhow::Result<Vec<Diagnostic>> {
        let mut diags = Vec::new();
        for page in pages {
            diags.extend(setoka_core::lint_page(page, &opts, &lint_opts)?);
        }
        // プロジェクト横断ルール（長音符ゆれの混在等）を合流させる
        diags.extend(setoka_core::lint_project(pages, &opts)?);
        Ok(diags)
    };

    let fixed = if fix {
        fix_until_stable(
            MAX_FIX_ROUNDS,
            || {
                Ok(setoka_core::build_source_pages(
                    &rc.content_dir,
                    &rc.config.input.ignore,
                    &opts,
                )?)
            },
            // 抑制された出現は --fix でも修正しない（報告と修正の非対称を防ぐ）。
            // invalid / unused の警告は fix を持たないので適用の側で素通しする
            |pages| {
                Ok(
                    setoka_core::apply_suppressions(collect(pages)?, pages, &opts, &lint_opts)
                        .diags,
                )
            },
            |page, fixed| super::rewrite_source(&page.src, &page.source, fixed),
        )?
    } else {
        FixOutcome::default()
    };
    let fixed_total = fixed.applied;
    let fixed_files: BTreeSet<PathBuf> = fixed
        .files
        .iter()
        .map(|src| src.strip_prefix(root).unwrap_or(src).to_owned())
        .collect();

    // 最終状態の報告（--fix 適用後に残った違反 = 機械修正できないもの）
    let pages = setoka_core::build_source_pages(&rc.content_dir, &rc.config.input.ignore, &opts)?;
    let mut diags = collect(&pages)?;
    diags.extend(diag::config_diagnostics(&rc));
    // frontmatter `lintDisable` のページ単位抑制（check と同じ漏斗）
    let setoka_core::SuppressionOutcome {
        diags,
        suppressed,
        disabled,
    } = setoka_core::apply_suppressions(diags, &pages, &opts, &lint_opts);

    // --fix の進捗は human 以外では stderr へ逃がす
    // （json は JSON オブジェクト以外を標準出力へ書かない契約のため）
    let progress_to_stdout = format == diag::Format::Human;
    for file in &fixed_files {
        let line = format!("修正: {}", file.display());
        if progress_to_stdout {
            outln!("{line}");
        } else {
            eprintln!("{line}");
        }
    }
    if fixed_total > 0 {
        let line = format!(
            "{fixed_total} 件を自動修正しました（{} ファイル）",
            fixed_files.len()
        );
        if progress_to_stdout {
            outln!("{line}");
        } else {
            eprintln!("{line}");
        }
    }

    diag::report(
        format,
        diags,
        &diag::Context {
            root,
            content_dir: &rc.content_dir,
            // 集計行は原稿の数を出す（合成した用語集ページは数えない）
            pages: pages.iter().filter(|p| !p.is_generated()).count(),
            suppressed,
            disabled,
            // lint は外部リンクを見ない（check の opt-in だけ）
            skipped: 0,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::collections::BTreeMap;
    use std::path::Path;

    use setoka_core::SourceSpan;

    /// content/ に原稿を置いたプロジェクト
    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (rel, text) in files {
            let path = dir.path().join("content").join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        dir
    }

    fn load(dir: &Path) -> anyhow::Result<Vec<Page>> {
        let opts = setoka_render::markdown_options(&setoka_config::Config::default());
        Ok(setoka_core::build_source_pages(
            &dir.join("content"),
            &[],
            &opts,
        )?)
    }

    /// 原稿で最初に出てくる `from` を `to` に置き換える修正つきの診断
    fn replace(page: &Page, from: &str, to: &str) -> Option<Diagnostic> {
        let start = page.source.find(from)?;
        let line_start = page.source[..start].rfind('\n').map_or(0, |i| i + 1);
        let line = page.source[..start].matches('\n').count() + 1;
        let col = start - line_start + 1;
        let span = SourceSpan {
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col + from.len(),
        };
        Some(
            Diagnostic::new(
                setoka_core::rules::TERM_VARIANT,
                page.rel.clone(),
                Some(span),
                "テスト",
            )
            .with_fix(to),
        )
    }

    /// 書いた回数を数えながら原稿へ書く
    fn writer(count: &Cell<usize>) -> impl FnMut(&Page, &str) -> anyhow::Result<()> + '_ {
        move |page, fixed| {
            count.set(count.get() + 1);
            std::fs::write(&page.src, fixed)?;
            Ok(())
        }
    }

    fn read(dir: &Path, rel: &str) -> String {
        std::fs::read_to_string(dir.join("content").join(rel)).unwrap()
    }

    #[test]
    fn 一度で直れば次のラウンドで止まる() {
        let dir = project(&[("a.md", "# a\n\nサーバを使う。\n")]);
        let writes = Cell::new(0);
        let outcome = fix_until_stable(
            MAX_FIX_ROUNDS,
            || load(dir.path()),
            |pages| {
                Ok(pages
                    .iter()
                    .filter_map(|p| replace(p, "サーバを", "サーバーを"))
                    .collect())
            },
            writer(&writes),
        )
        .unwrap();
        assert_eq!(read(dir.path(), "a.md"), "# a\n\nサーバーを使う。\n");
        assert_eq!(outcome.applied, 1);
        assert_eq!(outcome.rounds, 1);
        assert_eq!(writes.get(), 1);
        assert_eq!(
            outcome.files,
            BTreeSet::from([dir.path().join("content").join("a.md")])
        );
    }

    #[test]
    fn 修正が別のゆれを生んだら読み直して続けて直す() {
        // 1 回目で A → B、読み直した 2 回目で B → C（どちらも 1 回目の診断には出ない）
        let dir = project(&[("a.md", "# a\n\nA\n")]);
        let writes = Cell::new(0);
        let outcome = fix_until_stable(
            MAX_FIX_ROUNDS,
            || load(dir.path()),
            |pages| {
                Ok(pages
                    .iter()
                    .filter_map(|p| replace(p, "\nA", "\nB").or_else(|| replace(p, "\nB", "\nC")))
                    .collect())
            },
            writer(&writes),
        )
        .unwrap();
        assert_eq!(read(dir.path(), "a.md"), "# a\n\nC\n");
        assert_eq!(outcome.applied, 2);
        assert_eq!(outcome.rounds, 2);
        assert_eq!(writes.get(), 2);
    }

    #[test]
    fn 直しても終わらないときは上限の回数で止める() {
        // X と Y を入れ替え続ける（不動点が無い）
        let dir = project(&[("a.md", "# a\n\nX\n")]);
        let writes = Cell::new(0);
        let outcome = fix_until_stable(
            3,
            || load(dir.path()),
            |pages| {
                Ok(pages
                    .iter()
                    .filter_map(|p| replace(p, "\nX", "\nY").or_else(|| replace(p, "\nY", "\nX")))
                    .collect())
            },
            writer(&writes),
        )
        .unwrap();
        // X → Y → X → Y の 3 回で止まる
        assert_eq!(read(dir.path(), "a.md"), "# a\n\nY\n");
        assert_eq!(outcome.rounds, 3);
        assert_eq!(writes.get(), 3);
    }

    #[test]
    fn 適用できない修正と結果が同じ修正では書かない() {
        let dir = project(&[("a.md", "# a\n\n本文\n"), ("b.md", "# b\n\n本文\n")]);
        let writes = Cell::new(0);
        let outcome = fix_until_stable(
            MAX_FIX_ROUNDS,
            || load(dir.path()),
            |pages| {
                let mut diags = Vec::new();
                for page in pages {
                    if page.rel == Path::new("a.md") {
                        // 原稿に無い行を指す修正（apply_fixes が捨てる = 適用 0 件）
                        let span = SourceSpan {
                            start_line: 99,
                            start_col: 1,
                            end_line: 99,
                            end_col: 2,
                        };
                        diags.push(
                            Diagnostic::new(
                                setoka_core::rules::TERM_VARIANT,
                                page.rel.clone(),
                                Some(span),
                                "テスト",
                            )
                            .with_fix("x"),
                        );
                    } else {
                        // 置き換えても原稿と同じ
                        diags.extend(replace(page, "本文", "本文"));
                    }
                }
                Ok(diags)
            },
            writer(&writes),
        )
        .unwrap();
        assert_eq!(outcome, FixOutcome::default());
        assert_eq!(writes.get(), 0);
    }

    #[test]
    fn 合成した用語集のページには書かない() {
        // 用語集のページは原稿が無い（書くと content/glossary.md を新しく作ってしまう）
        let dir = project(&[("a.md", "# a\n\n本文\n")]);
        let mut config = setoka_config::Config::default();
        config.markdown.glossary.terms =
            BTreeMap::from([("SSR".to_string(), "Server-Side Rendering".to_string())]);
        let opts = setoka_render::markdown_options(&config);
        let generated = Cell::new(0);
        let writes = Cell::new(0);
        let outcome = fix_until_stable(
            MAX_FIX_ROUNDS,
            || {
                Ok(setoka_core::build_source_pages(
                    &dir.path().join("content"),
                    &[],
                    &opts,
                )?)
            },
            |pages| {
                let diags: Vec<Diagnostic> = pages
                    .iter()
                    .filter(|p| p.is_generated())
                    .filter_map(|p| replace(p, "# ", "## "))
                    .collect();
                generated.set(diags.len());
                Ok(diags)
            },
            writer(&writes),
        )
        .unwrap();
        // 用語集のページが修正つきの診断を受け取ったうえで、書かれていない
        assert_eq!(generated.get(), 1);
        assert_eq!(outcome, FixOutcome::default());
        assert_eq!(writes.get(), 0);
        assert!(!dir.path().join("content/glossary.md").exists());
    }

    #[test]
    fn 修正つきでない診断だけなら何も書かない() {
        let dir = project(&[("a.md", "# a\n\n本文\n")]);
        let writes = Cell::new(0);
        let outcome = fix_until_stable(
            MAX_FIX_ROUNDS,
            || load(dir.path()),
            |pages| {
                Ok(pages
                    .iter()
                    .map(|p| {
                        Diagnostic::new(setoka_core::rules::DUPLICATE_H1, p.rel.clone(), None, "x")
                    })
                    .collect())
            },
            writer(&writes),
        )
        .unwrap();
        assert_eq!(outcome, FixOutcome::default());
        assert_eq!(writes.get(), 0);
    }
}
