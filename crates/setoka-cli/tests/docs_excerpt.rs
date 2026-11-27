//! docs サイトの原稿（`docs/content/guide/code-and-math.md`）は、`docs/setoka.toml` の `[markdown]` とその下の表
//! （`[markdown.…]`）をインクルード（`file="setoka.toml" lines=…`）で引用し、同じ書き方を例としても見せている。
//! 次の 3 つを、setoka 自身の読み取りで確かめる（v0.22 Phase 107。docs/setoka.toml の行を動かすと原稿の引用がずれるので、
//! 行番号を文書に直書きせずここで縛る）。
//!
//! 1. 引用と例の範囲がそろっている（setoka が使う情報文字列の読み取り [`setoka_core::collect_include_specs`] で読む。
//!    属性は順不同・値の引用符の有無も setoka のとおり。例 = 4 つのバッククォートで囲んだ Markdown の中のフェンスの行も、
//!    その行を 1 つの Markdown として同じ関数に渡す）
//! 2. その範囲が `[markdown]` の表を漏れなく・余分なく指している（TOML は kabosu で読む）
//! 3. docs をビルドすると、引用のキャプションがその範囲になる（docs を一時ディレクトリへ写してビルドし、キャプションの
//!    文字列 `パス:開始-終わり` から取り出した範囲を、原稿から読んだ [`IncludeSpec`] の `path` と `lines` と直接比べる。
//!    期待値を描画と同じ `IncludeSpec::default_title` で作ると、その関数が壊れたときに見逃す）
//!
//! 以前はシェルや awk で原稿と TOML を読んでいたが、属性の順・値の引用符・複数行の文字列・エスケープした引用符付きの
//! キーなどで見逃しと誤りが続いた（PR #114 のレビュー指摘）。読み取りは setoka と kabosu の 1 実装に任せる。
//!
//! 失敗したら、メッセージの案内に従って `docs/setoka.toml` の表の並びか、`code-and-math.md` の `lines=`（引用と例。
//! 説明の文の例 `lines=27-47` も合わせる）を直す。

// 結合テストは書き込みの口の縛り（ワークスペースの clippy.toml。v0.22 Phase 106）の対象外
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use kabosu::{Document, Node, Span, Value};
use setoka_core::{IncludeSpec, MarkdownOptions, collect_include_specs, extract_fence_blocks};

fn docs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs")
}

const MANUSCRIPT: &str = "content/guide/code-and-math.md";
const PAGE: &str = "dist/guide/code-and-math/index.html";

/// Markdown の中の `setoka.toml` のインクルード（setoka の読み取りを通す）
fn setoka_toml_includes(markdown: &str, opts: &MarkdownOptions) -> Vec<IncludeSpec> {
    collect_include_specs(markdown, opts)
        .into_iter()
        .map(|r| r.spec)
        .filter(|spec| spec.path == "setoka.toml")
        .collect()
}

fn range_of(spec: &IncludeSpec) -> (usize, usize) {
    spec.lines
        .expect("setoka.toml の引用に lines= が無い（ファイル全体を引用している）")
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
            ranges.extend(setoka_toml_includes(&doc, opts).iter().map(range_of));
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

/// `range` の行にかかる、`markdown` 以外の最上位のキー（出てくる順）
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

/// HTML の中の `<figcaption>…</figcaption>` の中身（出てくる順）
fn figcaptions(html: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("<figcaption>") {
        rest = &rest[i + "<figcaption>".len()..];
        let Some(end) = rest.find("</figcaption>") else {
            break;
        };
        found.push(rest[..end].to_string());
        rest = &rest[end..];
    }
    found
}

/// `from` の中身を `to` へ写す（docs をビルドする一時ディレクトリを作る。前のビルドの出力とキャッシュは写さない）
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "dist" || name == ".setoka" {
            continue;
        }
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            copy_tree(&entry.path(), &to.join(&name));
        } else if kind.is_file() {
            fs::copy(entry.path(), to.join(&name)).unwrap();
        }
    }
}

#[test]
fn 原稿が引用する_setoka_toml_の範囲は_markdown_の表を指し_ビルドしたキャプションもその範囲() {
    let docs = docs_dir();
    let src = fs::read_to_string(docs.join("setoka.toml")).unwrap();
    let manuscript = fs::read_to_string(docs.join(MANUSCRIPT)).unwrap();
    let opts = MarkdownOptions::default();

    // 1. 引用と例の範囲がそろっている
    let includes = setoka_toml_includes(&manuscript, &opts);
    assert!(
        !includes.is_empty(),
        "code-and-math.md に setoka.toml の引用（file=\"setoka.toml\" lines=…）が無い"
    );
    let cited: Vec<(usize, usize)> = includes.iter().map(range_of).collect();
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

    // 2. その範囲が [markdown] の表を漏れなく・余分なく指している
    let expected = markdown_lines(&src).unwrap_or_else(|message| panic!("{message}"));
    assert_eq!(
        all[0], expected,
        "code-and-math.md の lines={}-{} が docs/setoka.toml の [markdown] の表（{}-{} 行目）と合わない。\
         lines={}-{} に直す（引用と例。説明の文の例も合わせる）",
        all[0].0, all[0].1, expected.0, expected.1, expected.0, expected.1
    );

    // 3. docs をビルドすると、引用のキャプションが setoka の作る文言（setoka.toml:行範囲）になる
    let work = tempfile::tempdir().unwrap();
    let root = work.path().join("docs");
    copy_tree(&docs, &root);
    let output = Command::new(env!("CARGO_BIN_EXE_setoka"))
        .arg("--root")
        .arg(&root)
        .args(["build", "-q"])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "docs のビルドに失敗した: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let html = fs::read_to_string(root.join(PAGE)).unwrap();
    let captions: Vec<String> = figcaptions(&html)
        .into_iter()
        .filter(|c| c.starts_with("setoka.toml"))
        .collect();
    // キャプションの文字列から行の範囲を取り出し、原稿から読んだ IncludeSpec の path と lines と直接比べる。
    // 期待値を描画と同じ IncludeSpec::default_title で作ると、その関数が壊れたときに両方が同じように変わって
    // 見逃す（PR #114 のレビュー指摘）
    let rendered: Vec<Option<(String, (usize, usize))>> =
        captions.iter().map(|c| parse_caption(c)).collect();
    let wanted: Vec<Option<(String, (usize, usize))>> = includes
        .iter()
        .map(|spec| Some((spec.path.clone(), range_of(spec))))
        .collect();
    assert_eq!(
        rendered, wanted,
        "code-and-math のページの setoka.toml の引用のキャプション（{captions:?}）が、原稿の引用（{cited:?}）と違う"
    );
}

/// キャプション `パス:開始-終わり`（1 行だけなら `パス:行`）の、パスと行の範囲。この形でなければ `None`
fn parse_caption(caption: &str) -> Option<(String, (usize, usize))> {
    let (path, range) = caption.rsplit_once(':')?;
    let (start, end) = match range.split_once('-') {
        Some((s, e)) => (s.parse().ok()?, e.parse().ok()?),
        None => {
            let n = range.parse().ok()?;
            (n, n)
        }
    };
    Some((path.to_string(), (start, end)))
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

/// 引用と例の範囲は、属性の順と値の引用符の有無に依らず setoka の読み取りで拾う（PR #114 のレビュー指摘）
#[test]
fn 引用と例の範囲は属性の順と値の引用符に依らず読む() {
    let opts = MarkdownOptions::default();
    let markdown = "````markdown\n```toml lines=27-44 file=\"setoka.toml\"\n````\n\n```toml file=setoka.toml lines=\"27-47\"\n```\n";
    let ranges: Vec<(usize, usize)> = setoka_toml_includes(markdown, &opts)
        .iter()
        .map(range_of)
        .collect();
    assert_eq!(ranges, [(27, 47)]);
    assert_eq!(example_ranges(markdown, &opts), [(27, 44)]);
}

#[test]
fn キャプションの取り出し() {
    let html = "<figure><figcaption>setoka.toml:27-47</figcaption><pre></pre></figure>\
                <figcaption>図 1: 依存</figcaption>";
    assert_eq!(figcaptions(html), ["setoka.toml:27-47", "図 1: 依存"]);
    assert_eq!(
        parse_caption("setoka.toml:27-47"),
        Some(("setoka.toml".to_string(), (27, 47)))
    );
    assert_eq!(
        parse_caption("setoka.toml:7"),
        Some(("setoka.toml".to_string(), (7, 7)))
    );
    assert_eq!(parse_caption("setoka.toml"), None, "範囲の無いキャプション");
    assert_eq!(parse_caption("setoka.toml:27-"), None);
}
