//! docs サイトの原稿（`docs/content/guide/code-and-math.md`）は、`docs/setoka.toml` の `[markdown]` とその下の表
//! （`[markdown.…]`）をインクルード（`file="setoka.toml" lines=…`）で引用し、同じ書き方を例としても見せている。
//! 引用の範囲が `[markdown]` の表を漏れなく・余分なく指しているか、例の範囲が実際の引用と同じかを確かめる
//! （v0.22 Phase 107。docs/setoka.toml の行を動かすと原稿の引用がずれるので、行番号を文書に直書きせずここで縛る）。
//!
//! - 引用は setoka が使う情報文字列の読み取り（[`setoka_core::collect_include_specs`]。属性は順不同）で読む。
//!   例（4 つのバッククォートで囲んだ Markdown の中のフェンスの行）も、その行を 1 つの Markdown として同じ関数に渡す
//!   （`file=` の直後の `lines=` だけを文字列で拾うと、`lines=27-44 file="setoka.toml"` の順に書いた例を見逃した）
//! - TOML は kabosu で読む（awk で読むと、複数行の文字列の中の `[lint]`・エスケープした引用符付きのキー・4 個の引用符で
//!   終わる文字列などで見逃した）。以上は PR #114 のレビュー指摘
//!
//! 失敗したら、メッセージの案内に従って `docs/setoka.toml` の表の並びか、`code-and-math.md` の `lines=`（引用と例。
//! 説明の文の例 `lines=27-47` も合わせる）を直す。

use std::path::{Path, PathBuf};

use kabosu::{Document, Node, Span, Value};
use setoka_core::{MarkdownOptions, collect_include_specs, extract_fence_blocks};

fn docs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs")
}

/// Markdown の中の `setoka.toml` のインクルードの範囲（setoka の読み取りを通す）
fn setoka_toml_ranges(markdown: &str, opts: &MarkdownOptions) -> Vec<(usize, usize)> {
    collect_include_specs(markdown, opts)
        .into_iter()
        .filter(|r| r.spec.path == "setoka.toml")
        .map(|r| {
            r.spec
                .lines
                .expect("setoka.toml の引用に lines= が無い（ファイル全体を引用している）")
        })
        .collect()
}

/// コードブロックの中に例として書いたフェンスの行（```toml file=… など）の `setoka.toml` の範囲
fn example_ranges(markdown: &str, opts: &MarkdownOptions) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    for block in extract_fence_blocks(markdown, opts) {
        for line in block.body.lines() {
            let trimmed = line.trim_start();
            let fence: String = trimmed
                .chars()
                .take_while(|c| *c == '`' || *c == '~')
                .collect();
            if fence.len() < 3 {
                continue;
            }
            // 例の 1 行を、閉じたフェンスとして同じ読み取りに渡す
            let doc = format!("{trimmed}\n{fence}\n");
            ranges.extend(setoka_toml_ranges(&doc, opts));
        }
    }
    ranges
}

/// キーと値の位置を、表・配列・インラインテーブルの中まで集める
fn collect_node(node: &Node, spans: &mut Vec<Span>) {
    spans.push(node.span());
    match node.value() {
        Value::Table(table) => {
            for entry in table.entries() {
                spans.push(entry.key_span());
                collect_node(entry.node(), spans);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_node(item, spans);
            }
        }
        _ => {}
    }
}

/// 最上位のキー `key` に属するもの（見出しのキー・その下の表・キー・値）の位置
fn spans_of(doc: &Document, key: &str) -> Vec<Span> {
    let entry = doc
        .root()
        .get(key)
        .unwrap_or_else(|| panic!("[{key}] が無い"));
    let mut spans = vec![entry.key_span()];
    collect_node(entry.node(), &mut spans);
    spans
}

/// 位置の集まりが占める行（1 始まり。最初の位置の行から、最後の位置の最後の文字の行まで）
fn line_range(doc: &Document, spans: &[Span]) -> (usize, usize) {
    let start = spans.iter().map(|s| s.start).min().unwrap();
    let end = spans.iter().map(|s| s.end).max().unwrap();
    (
        doc.line_col(start).line,
        doc.line_col(end.saturating_sub(1)).line,
    )
}

/// `range` の行にかかる、`markdown` 以外の最上位のキー（重複なし・出てくる順）
fn others_within(doc: &Document, range: (usize, usize)) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    for entry in doc.root().entries().filter(|e| e.key() != "markdown") {
        for span in spans_of(doc, entry.key()) {
            let line = doc.line_col(span.start).line;
            if line >= range.0 && line <= range.1 {
                found.push((entry.key().to_string(), line));
                break;
            }
        }
    }
    found
}

/// `docs/setoka.toml` の `[markdown]` の表が占める行。間に別の表が挟まって一続きでなければ、表の並びを直すよう案内する
fn markdown_lines(src: &str) -> Result<(usize, usize), String> {
    let doc = Document::parse(src).map_err(|e| format!("docs/setoka.toml を読めない: {e:?}"))?;
    let range = line_range(&doc, &spans_of(&doc, "markdown"));
    let others = others_within(&doc, range);
    if !others.is_empty() {
        let list: Vec<String> = others
            .iter()
            .map(|(key, line)| format!("[{key}]（{line} 行目）"))
            .collect();
        return Err(format!(
            "docs/setoka.toml の [markdown] の表が一続きになっていない（{}-{} 行目の間に {} がある）。\
             [markdown.…] の表を [markdown] の近くへ移して一続きにしてから、code-and-math.md の lines= を直す",
            range.0,
            range.1,
            list.join("・")
        ));
    }
    Ok(range)
}

#[test]
fn 原稿が引用する_setoka_toml_の範囲は_markdown_の表を漏れなく余分なく指す() {
    let docs = docs_dir();
    let src = std::fs::read_to_string(docs.join("setoka.toml")).unwrap();
    let manuscript = std::fs::read_to_string(docs.join("content/guide/code-and-math.md")).unwrap();
    let opts = MarkdownOptions::default();

    let cited = setoka_toml_ranges(&manuscript, &opts);
    assert!(
        !cited.is_empty(),
        "code-and-math.md に setoka.toml の引用（file=\"setoka.toml\" lines=…）が無い"
    );
    let examples = example_ranges(&manuscript, &opts);
    assert!(
        !examples.is_empty(),
        "code-and-math.md に setoka.toml の引用の書き方の例が無い"
    );
    let all: Vec<(usize, usize)> = cited.iter().chain(&examples).copied().collect();
    assert!(
        all.iter().all(|r| *r == all[0]),
        "code-and-math.md の setoka.toml の引用と例の範囲がそろっていない（引用 {cited:?}・例 {examples:?}）"
    );

    let expected = markdown_lines(&src).unwrap_or_else(|message| panic!("{message}"));
    assert_eq!(
        all[0], expected,
        "code-and-math.md の lines={}-{} が docs/setoka.toml の [markdown] の表（{}-{} 行目）と合わない。\
         lines={}-{} に直す（引用と例。説明の文の例も合わせる）",
        all[0].0, all[0].1, expected.0, expected.1, expected.0, expected.1
    );
}

/// 表の範囲の求め方が、awk で読んでいたときに見逃した書き方に依らないこと（PR #114 のレビューの再現）
#[test]
fn 表の範囲の判定は_toml_の書き方に依らない() {
    // 複数行の文字列の中の `[lint]` の行は見出しではない
    let src = "[markdown]\ngfm = true\n[markdown.glossary.terms]\nA = \"\"\"\na\n[lint]\nb\"\"\"\nB = \"b\"\n\n[lint]\nx = 1\n";
    assert_eq!(markdown_lines(src), Ok((1, 8)));
    // [markdown] より前の [markdown.math]・インデントした見出し・見出しの末尾のコメント
    let src = "[markdown.math]\nenabled = true\n\n  [markdown] # 設定\ngfm = true\n";
    assert_eq!(markdown_lines(src), Ok((1, 5)));
    // エスケープした引用符付きのキーも同じ表。間に別の表があれば一続きにするよう案内する
    let src =
        "[markdown]\ngfm = true\n\n[lint]\nx = 1\n\n[\"mark\\u0064own\".math]\nenabled = true\n";
    let err = markdown_lines(src).unwrap_err();
    assert!(err.contains("一続きになっていない"), "{err}");
    assert!(err.contains("[lint]（4 行目）"), "{err}");
    // 4 個の引用符で終わる複数行の文字列と、同じ行の配列の閉じ括弧
    let src =
        "[markdown]\narr = [\"\"\"a\"\"\"\"]\n\n[lint]\nx = 1\n\n[markdown.math]\nenabled = true\n";
    assert!(
        markdown_lines(src)
            .unwrap_err()
            .contains("[lint]（4 行目）")
    );
}

/// 引用と例の範囲は、属性の順に依らず setoka の読み取りで拾う（PR #114 のレビュー指摘）
#[test]
fn 引用と例の範囲は属性の順に依らず読む() {
    let opts = MarkdownOptions::default();
    let markdown = "````markdown\n```toml lines=27-44 file=\"setoka.toml\"\n````\n\n```toml file=\"setoka.toml\" lines=27-47\n```\n";
    assert_eq!(setoka_toml_ranges(markdown, &opts), [(27, 47)]);
    assert_eq!(example_ranges(markdown, &opts), [(27, 44)]);
}
