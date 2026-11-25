//! docs サイトの原稿（`docs/content/guide/code-and-math.md`）は、`docs/setoka.toml` の `[markdown]` とその下の表
//! （`[markdown.…]`）をインクルードの `lines=` で引用している。その範囲が `[markdown]` の表を漏れなく・余分なく指して
//! いるかを、kabosu で読んだキーと値の位置で確かめる（v0.22 Phase 107。docs/setoka.toml の行を動かすと原稿の引用が
//! ずれるので、行番号を文書に直書きせずここで縛る）。
//!
//! 以前は `scripts/docs-gates.sh` が awk で TOML を読んで確かめていたが、複数行の文字列の中の `[lint]`・エスケープした
//! 引用符付きのキー（`"markdown"`）・4 個の引用符で終わる文字列などの有効な書き方で、引用漏れを見逃した
//! （PR #114 のレビュー指摘）。TOML の解釈はパーサに任せる。
//!
//! 失敗したら、メッセージに出る範囲で `code-and-math.md` の `lines=`（引用の 2 か所と、説明の文の例）を直す。

use std::path::{Path, PathBuf};

use kabosu::{Document, Node, Span, Table, Value};

fn docs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs")
}

/// 原稿の `file="setoka.toml" lines=A-B` の範囲をすべて取り出す
fn cited_ranges(markdown: &str) -> Vec<(usize, usize)> {
    const NEEDLE: &str = "file=\"setoka.toml\" lines=";
    let mut ranges = Vec::new();
    let mut rest = markdown;
    while let Some(i) = rest.find(NEEDLE) {
        rest = &rest[i + NEEDLE.len()..];
        let spec: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        let (a, b) = spec
            .split_once('-')
            .unwrap_or_else(|| panic!("lines= が A-B の形ではありません: {spec}"));
        ranges.push((a.parse().unwrap(), b.parse().unwrap()));
    }
    ranges
}

/// キーと値の位置を、表と配列の中まで集める
fn collect_spans(table: &Table, spans: &mut Vec<Span>) {
    for entry in table.entries() {
        spans.push(entry.key_span());
        collect_node(entry.node(), spans);
    }
}

fn collect_node(node: &Node, spans: &mut Vec<Span>) {
    spans.push(node.span());
    match node.value() {
        Value::Table(table) => collect_spans(table, spans),
        Value::Array(items) => {
            for item in items {
                collect_node(item, spans);
            }
        }
        _ => {}
    }
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

#[test]
fn 原稿が引用する_setoka_toml_の範囲は_markdown_の表を漏れなく余分なく指す() {
    let docs = docs_dir();
    let src = std::fs::read_to_string(docs.join("setoka.toml")).unwrap();
    let doc = Document::parse(&src).expect("docs/setoka.toml を読めない");
    let manuscript = std::fs::read_to_string(docs.join("content/guide/code-and-math.md")).unwrap();

    let ranges = cited_ranges(&manuscript);
    assert!(
        !ranges.is_empty(),
        "code-and-math.md に setoka.toml の引用（file=\"setoka.toml\" lines=）が無い"
    );
    assert!(
        ranges.iter().all(|r| *r == ranges[0]),
        "code-and-math.md の setoka.toml の引用の範囲がそろっていない: {ranges:?}"
    );
    let cited = ranges[0];

    // [markdown] の表に属するもの（見出しのキー・その下の表・キー・値。配列とインラインテーブルの中も）の行
    let markdown = doc
        .root()
        .get("markdown")
        .expect("docs/setoka.toml に [markdown] が無い");
    let mut spans = vec![markdown.key_span()];
    collect_node(markdown.node(), &mut spans);
    let expected = line_range(&doc, &spans);
    assert_eq!(
        cited, expected,
        "code-and-math.md の lines={}-{} が docs/setoka.toml の [markdown] の表（{}-{} 行目）と合わない。\
         lines={}-{} に直す",
        cited.0, cited.1, expected.0, expected.1, expected.0, expected.1
    );

    // 範囲の中に、[markdown] の外のキー・値が無い（範囲の前後の表を引用に巻き込まない）
    for entry in doc.root().entries().filter(|e| e.key() != "markdown") {
        let mut other = vec![entry.key_span()];
        collect_node(entry.node(), &mut other);
        for span in other {
            let line = doc.line_col(span.start).line;
            assert!(
                line < cited.0 || line > cited.1,
                "code-and-math.md の lines={}-{} の中（{line} 行目）に [markdown] の外の `{}` がある",
                cited.0,
                cited.1,
                entry.key()
            );
        }
    }
}

/// 上の判定が、awk で読んでいたときに見逃した書き方でも漏れを見つけること（PR #114 のレビューの再現）
#[test]
fn 引用の範囲の判定は_toml_の書き方に依らない() {
    let markdown_lines = |src: &str| {
        let doc = Document::parse(src).unwrap();
        let markdown = doc.root().get("markdown").unwrap();
        let mut spans = vec![markdown.key_span()];
        collect_node(markdown.node(), &mut spans);
        line_range(&doc, &spans)
    };
    // 複数行の文字列の中の `[lint]` の行は見出しではない
    let src = "[markdown]\ngfm = true\n[markdown.glossary.terms]\nA = \"\"\"\na\n[lint]\nb\"\"\"\nB = \"b\"\n\n[lint]\nx = 1\n";
    assert_eq!(markdown_lines(src), (1, 8));
    // エスケープした引用符付きのキーも同じ表
    let src =
        "[markdown]\ngfm = true\n\n[lint]\nx = 1\n\n[\"mark\\u0064own\".math]\nenabled = true\n";
    assert_eq!(markdown_lines(src), (1, 8));
    // 4 個の引用符で終わる複数行の文字列と、同じ行の配列の閉じ括弧
    let src =
        "[markdown]\narr = [\"\"\"a\"\"\"\"]\n\n[lint]\nx = 1\n\n[markdown.math]\nenabled = true\n";
    assert_eq!(markdown_lines(src), (1, 8));
    // [markdown] より前の [markdown.math]・インデントした見出し・見出しの末尾のコメント
    let src = "[markdown.math]\nenabled = true\n\n  [markdown] # 設定\ngfm = true\n";
    assert_eq!(markdown_lines(src), (1, 5));
}
