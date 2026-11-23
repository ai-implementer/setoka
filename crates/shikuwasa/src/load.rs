//! saphyr で読み、saphyr が見ない検査（別名の展開の上限・同名のキー・コアタグ）を掛ける

use std::collections::{HashMap, HashSet};

use saphyr::{AnnotatedMapping, MarkedYaml, Tag, YamlData, YamlLoader};
use saphyr_parser::{Event, Marker, Parser, ScanError, Span, SpannedEventReceiver};

use crate::MAX_ALIAS_NODES;
use crate::scalar::{ScalarValue, resolve_tagged};

/// saphyr の位置付きのノード
pub(crate) type Raw<'a> = MarkedYaml<'a>;

/// 読み込みのエラー（位置は入力の中のバイト位置）
pub(crate) struct LoadError {
    pub offset: usize,
    pub message: String,
}

/// saphyr の位置（文字数）を入力の中のバイト位置にする。ASCII だけの入力は文字数がそのまま
/// バイト位置になる。ほかは各文字の開始バイトの表を 1 回だけ作る（ノードごとに先頭から
/// 数え直すと、ノードの多い文書で入力の長さ × ノード数になる）
pub(crate) struct Positions<'t> {
    text: &'t str,
    starts: Option<Vec<usize>>,
}

impl<'t> Positions<'t> {
    pub(crate) fn new(text: &'t str) -> Self {
        let starts = (!text.is_ascii()).then(|| text.char_indices().map(|(i, _)| i).collect());
        Self { text, starts }
    }

    pub(crate) fn byte(&self, mark: Marker) -> usize {
        let index = mark.index();
        match &self.starts {
            None => index.min(self.text.len()),
            Some(starts) => starts.get(index).copied().unwrap_or(self.text.len()),
        }
    }
}

/// 1 つの文書を位置付きの木として読む。文書が無い（空・コメントだけ）なら `None`
pub(crate) fn load<'a>(text: &'a str, pos: &Positions<'_>) -> Result<Option<Raw<'a>>, LoadError> {
    let mut receiver = Budgeted::default();
    receiver.loader.early_parse(false);
    // saphyr 0.3.1 からは loader 自身も別名の展開に上限（既定 10 万ノード）を掛け、超えると英語の
    // 構文エラーにする。上限を同じ値にそろえる。saphyr はマッピングのキーを 1 ノードと数えるので、
    // 数は Budgeted 以下になり、上限には必ず Budgeted が先に当たる（文言は日本語のまま。
    // setoka の PR #79 のレビュー指摘。既定のままだと 10 万を超え 20 万以下の文書が通らなかった）
    receiver.loader.alias_limit(MAX_ALIAS_NODES);
    let scanned = Parser::new_from_str(text).load(&mut receiver, true);
    // 上限を超えた後は loader へ渡していないので、loader 側の結果より先に見る
    if let Some(mark) = receiver.exceeded {
        return Err(LoadError {
            offset: pos.byte(mark),
            message: format!(
                "別名（`*名前`）で展開する値が多すぎます（上限 {MAX_ALIAS_NODES} 個）"
            ),
        });
    }
    if let Some(e) = scanned.err().as_ref().or(receiver.loader.error()) {
        return Err(LoadError {
            offset: pos.byte(*e.marker()),
            message: scan_message(e),
        });
    }
    let mut docs = receiver.loader.into_documents();
    if let Some(second) = docs.get(1) {
        return Err(LoadError {
            offset: pos.byte(second.span.start),
            message: "YAML の文書が複数あります（`---` で区切った 2 つ目以降は読めません）"
                .to_string(),
        });
    }
    let root = docs.pop();
    if let Some(Err((mark, message))) = root.as_ref().map(validate) {
        return Err(LoadError {
            offset: pos.byte(mark),
            message,
        });
    }
    Ok(root)
}

/// 重複キーの文言（saphyr が見つけた重複と [`validate`] が見つけた重複で同じにする）
pub(crate) const DUPLICATE_KEY: &str =
    "キーが重複しています（同じキーは 1 回だけ書けます。引用符の有無は区別しません）";

/// saphyr が見ない 2 つを確かめる: 引用符やタグだけが違う同名のキーと、明示のコアタグに
/// 合わない値。キーはスカラーの原文で比べる（原文が同じなら同じキーとして扱われるため）
fn validate(node: &Raw<'_>) -> Result<(), (Marker, String)> {
    match view(node) {
        View::Scalar { text, tag, .. } => {
            if let Some(message) = tag_problem(text, tag) {
                return Err((node.span.start, message));
            }
        }
        View::Sequence(items) => {
            for item in items {
                validate(item)?;
            }
        }
        View::Mapping(map) => {
            let mut seen = HashSet::new();
            for (key, value) in map {
                if let View::Scalar { text, .. } = view(key)
                    && !seen.insert(text)
                {
                    return Err((key.span.start, DUPLICATE_KEY.to_string()));
                }
                validate(key)?;
                validate(value)?;
            }
        }
        View::Other => {}
    }
    Ok(())
}

/// 明示のコアタグに合わない値の文言（`!!int "x"`・`!!bool yes`）
fn tag_problem(text: &str, tag: Option<&Tag>) -> Option<String> {
    let suffix = core_tag(tag)?;
    let resolved = resolve_tagged(text, suffix)?;
    let (fits, kind) = match suffix {
        "bool" => (matches!(resolved, ScalarValue::Bool(_)), "真偽値"),
        "int" => (
            matches!(resolved, ScalarValue::Int(_) | ScalarValue::UInt(_)),
            "整数",
        ),
        "float" => (matches!(resolved, ScalarValue::Float(_)), "小数"),
        _ => (resolved == ScalarValue::Null, "値なし（null）"),
    };
    (!fits).then(|| format!("`!!{suffix}` のタグが付いた `{text}` は{kind}として読めません"))
}

/// コアタグ（`tag:yaml.org,2002:` の `!!bool` など）の名前
pub(crate) fn core_tag(tag: Option<&Tag>) -> Option<&str> {
    tag.filter(|t| t.is_yaml_core_schema())
        .map(|t| t.suffix.as_str())
}

/// ノードの中身（タグ付きの集合は中を見る。位置はタグの外側のノードを使うこと =
/// saphyr はタグの中のノードに位置を付けない）
pub(crate) enum View<'n, 'a> {
    Scalar {
        text: &'n str,
        tag: Option<&'n Tag>,
    },
    Sequence(&'n [Raw<'a>]),
    Mapping(&'n AnnotatedMapping<'a, Raw<'a>>),
    /// 読み込みでは作られない（解決できない別名は構文エラー・`early_parse` は無効）
    Other,
}

pub(crate) fn view<'n, 'a>(node: &'n Raw<'a>) -> View<'n, 'a> {
    match &node.data {
        YamlData::Representation(text, _, tag) => View::Scalar {
            text,
            tag: tag.as_deref(),
        },
        YamlData::Sequence(items) => View::Sequence(items),
        YamlData::Mapping(map) => View::Mapping(map),
        YamlData::Tagged(_, inner) => view(inner),
        _ => View::Other,
    }
}

/// 別名の展開を数えながら saphyr の loader へイベントを渡す。
/// アンカーごとに部分木のノード数（入れ子の別名の展開を含む）を覚え、別名 1 つで
/// その数だけ足す。上限を超えたら以降を渡さない（loader が部分木を複製しない）
#[derive(Default)]
struct Budgeted<'a> {
    loader: YamlLoader<'a, Raw<'a>>,
    /// ここまでのノード数（別名の展開を含む）
    nodes: usize,
    /// 別名で展開したノード数
    via_alias: usize,
    /// アンカー ID → 部分木のノード数
    anchors: HashMap<usize, usize>,
    /// 開いている集合の（アンカー ID、開いた時点のノード数）
    open: Vec<(usize, usize)>,
    exceeded: Option<Marker>,
}

impl<'a> SpannedEventReceiver<'a> for Budgeted<'a> {
    fn on_event(&mut self, ev: Event<'a>, span: Span) {
        if self.exceeded.is_some() {
            return;
        }
        match &ev {
            Event::Scalar(_, _, anchor, _) => {
                self.nodes += 1;
                if *anchor > 0 {
                    self.anchors.insert(*anchor, 1);
                }
            }
            Event::SequenceStart(anchor, _) | Event::MappingStart(anchor, _) => {
                self.open.push((*anchor, self.nodes));
                self.nodes += 1;
            }
            Event::SequenceEnd | Event::MappingEnd => {
                if let Some((anchor, start)) = self.open.pop()
                    && anchor > 0
                {
                    self.anchors.insert(anchor, self.nodes - start);
                }
            }
            Event::Alias(id) => {
                let size = self.anchors.get(id).copied().unwrap_or(1);
                self.nodes += size;
                self.via_alias += size;
                if self.via_alias > MAX_ALIAS_NODES {
                    self.exceeded = Some(span.start);
                    return;
                }
            }
            _ => {}
        }
        self.loader.on_event(ev, span);
    }
}

/// saphyr の構文エラーの文言を日本語にする（よく当たるものだけ。ほかは原文を添える）
fn scan_message(e: &ScanError) -> String {
    let info = e.info();
    let message = match info {
        "duplicated key in mapping" => DUPLICATE_KEY,
        "while parsing a flow sequence, expected ',' or ']'" => {
            "`[` で始めた配列が閉じていません（`]` がありません）"
        }
        "while parsing a flow mapping, did not find expected ',' or '}'" => {
            "`{` で始めたマッピングが閉じていません（`}` がありません）"
        }
        "while scanning a quoted scalar, found unexpected end of stream" => {
            "引用符で始めた文字列が閉じていません"
        }
        "mapping values are not allowed in this context" => {
            "値の中に `: ` があります（`: ` を含む値は引用符で囲んでください）"
        }
        "while parsing a block mapping, did not find expected key" => {
            "キーの字下げがそろっていません"
        }
        "while parsing a quoted scalar, found unknown escape character" => {
            "`\"` で囲んだ文字列に使えない `\\` があります（`\\` を含む値は `'...'` で囲んでください）"
        }
        "while parsing node, found unknown anchor" => "定義されていない別名（`*名前`）があります",
        "recursion limit exceeded" => "入れ子が深すぎます",
        // saphyr は `title:<タブ>値` を通さない（置き換え前の serde_yaml_ng は通していた）
        "':' must be followed by a valid YAML whitespace" => {
            "`:` の後にタブがあります（`:` の後は空白にしてください）"
        }
        "'-' must be followed by a valid YAML whitespace" => {
            "`-` の後にタブがあります（`-` の後は空白にしてください）"
        }
        _ if info.contains("tab") => "タブで字下げしています（字下げは空白で書いてください）",
        _ => return format!("YAML の構文エラーです（{info}）"),
    };
    message.to_string()
}
