//! # shikuwasa
//!
//! YAML を位置付きの木として安全に読む。パーサは [saphyr](https://crates.io/crates/saphyr)
//! （YAML 1.2）で、その上に次の 4 つを足す。
//!
//! - **別名の展開に上限を掛ける** — 入れ子の別名（`*x`）で指数的に膨らむ文書を
//!   [`MAX_ALIAS_NODES`] で止める（saphyr の loader だけでは展開を数えきれない）
//! - **同名のキーをエラーにする** — 引用符やタグだけが違う同名のキー（`draft` と `"draft"`）も
//!   重複とみる（後の値が黙って勝たない）
//! - **スカラーの型を決める** — YAML 1.2 の core schema で解釈する。境界は serde_yaml_ng と同じで、
//!   先頭 0 の 10 進数（`010`）・`_` 区切り・YAML 1.1 の `yes` / `on` は文字列。明示のコアタグ
//!   （`!!bool` など）は引用符より先に見て、合わない値はエラーにする（[`resolve_plain`]）
//! - **位置はバイト** — ノードとエラーの位置は入力の中のバイト位置（saphyr の位置は文字数）。
//!   [`line_col`] で 1 始まりの行とバイト基準の列にできる
//!
//! エラーの文言は日本語（Markdown の frontmatter を書く人にそのまま見せるため）。
//!
//! ```
//! use shikuwasa::{Kind, ScalarValue};
//!
//! let text = "title: 設計書\norder: 2\ndraft: yes\n";
//! let root = shikuwasa::parse(text).unwrap().unwrap();
//! let Kind::Mapping(entries) = &root.kind else { panic!() };
//! let (key, value) = &entries[1];
//! assert_eq!(key.as_scalar().unwrap().text, "order");
//! assert_eq!(value.as_scalar().unwrap().value, ScalarValue::Int(2));
//! // YAML 1.2 の `yes` は真偽値ではなく文字列
//! assert_eq!(entries[2].1.as_scalar().unwrap().value, ScalarValue::Str);
//! assert_eq!(shikuwasa::line_col(text, value.offset), (2, 8));
//!
//! let e = shikuwasa::parse("a: 1\n\"a\": 2\n").unwrap_err();
//! assert_eq!((e.line, e.col), (2, 1));
//! ```
//!
//! feature `json` で、YAML（JSON を含む）を `serde_json::Value` へ写す [`to_json`] が使える。

#![forbid(unsafe_code)]

mod load;
mod scalar;

use saphyr::{ScalarStyle, YamlData};

use load::{Positions, Raw};
pub use scalar::{ScalarValue, resolve_plain};

/// 別名で展開するノード数の上限（文書全体の合計）。共通部品を別名で使い回す程度の文書は収まり、
/// 入れ子の別名で膨らむ文書は止まる（10 個ずつ 6 段で 100 万ノード）
pub const MAX_ALIAS_NODES: usize = 200_000;

/// 読めなかった理由と位置。`offset` は入力の中のバイト位置、`line` と `col` は 1 始まりの行と
/// バイト基準の列（[`line_col`] と同じ数え方）
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Error {
    pub offset: usize,
    pub line: usize,
    pub col: usize,
    /// 日本語の説明
    pub message: String,
}

impl Error {
    fn at(text: &str, offset: usize, message: String) -> Self {
        let (line, col) = line_col(text, offset);
        Self {
            offset,
            line,
            col,
            message,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}（{} 行 {} 列）", self.message, self.line, self.col)
    }
}

impl std::error::Error for Error {}

/// 位置付きのノード
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Node {
    /// 入力の中のバイト位置（ノードの始まり。タグ付きのノードはタグの位置）
    pub offset: usize,
    /// 付いていたタグ（`!!str`・`!custom` など）。集合（配列・マッピング）に付けたコアタグ
    /// （`!!map`・`!!set` など）は saphyr が残さないので `None`
    pub tag: Option<Tag>,
    pub kind: Kind,
}

/// ノードの中身
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Scalar(Scalar),
    Sequence(Vec<Node>),
    /// キーと値の組（書いた順。同名のキーは読み込みがエラーにしている）
    Mapping(Vec<(Node, Node)>),
    /// saphyr が値を作れなかったもの（作っている途中のアンカーを指す循環した別名 `&a [*a]` の `*a` など）。
    /// 値なし（null）とは区別する。[`to_json`] では null になる
    Unresolved,
}

/// スカラー
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Scalar {
    /// 原文（引用符の中身・ブロックの中身。`200` と書いたキーも `"200"`）
    pub text: String,
    pub style: Style,
    /// 型の解釈（書き方とタグから決めたもの）
    pub value: ScalarValue,
}

/// スカラーの書き方
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// 引用符なし
    Plain,
    /// `'...'`
    SingleQuoted,
    /// `"..."`
    DoubleQuoted,
    /// `|`
    Literal,
    /// `>`
    Folded,
}

/// タグ（`!!str` は handle が `tag:yaml.org,2002:`、suffix が `str`）
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Tag {
    pub handle: String,
    pub suffix: String,
}

impl Tag {
    /// YAML の core schema のタグ（`!!str`・`!!int` など）か
    pub fn is_core(&self) -> bool {
        self.handle == "tag:yaml.org,2002:"
    }
}

impl Node {
    pub fn as_scalar(&self) -> Option<&Scalar> {
        match &self.kind {
            Kind::Scalar(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_sequence(&self) -> Option<&[Node]> {
        match &self.kind {
            Kind::Sequence(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_mapping(&self) -> Option<&[(Node, Node)]> {
        match &self.kind {
            Kind::Mapping(entries) => Some(entries),
            _ => None,
        }
    }

    /// 値なし（`~`・`null`・空）のスカラーか
    pub fn is_null(&self) -> bool {
        self.as_scalar()
            .is_some_and(|s| s.value == ScalarValue::Null)
    }
}

/// 1 つの文書を位置付きの木として読む。文書が無い（空・コメントだけ）なら `None`。
/// `---` で区切った 2 つ目の文書があればエラー
pub fn parse(text: &str) -> Result<Option<Node>, Error> {
    let pos = Positions::new(text);
    let root = load::load(text, &pos).map_err(|e| Error::at(text, e.offset, e.message))?;
    Ok(root.as_ref().map(|root| convert(root, &pos)))
}

/// saphyr の木を自前の木にする（位置はバイトに直す。スカラーの型はここで決める）
fn convert(node: &Raw<'_>, pos: &Positions<'_>) -> Node {
    let offset = pos.byte(node.span.start);
    let leaf = |kind| Node {
        offset,
        tag: None,
        kind,
    };
    match &node.data {
        YamlData::Representation(text, style, tag) => {
            let tag = tag.as_deref();
            let value = scalar::resolve(text, *style == ScalarStyle::Plain, load::core_tag(tag));
            Node {
                offset,
                tag: tag.map(|t| Tag {
                    handle: t.handle.clone(),
                    suffix: t.suffix.clone(),
                }),
                kind: Kind::Scalar(Scalar {
                    text: text.to_string(),
                    style: style_of(*style),
                    value,
                }),
            }
        }
        YamlData::Sequence(items) => leaf(Kind::Sequence(
            items.iter().map(|item| convert(item, pos)).collect(),
        )),
        YamlData::Mapping(map) => leaf(Kind::Mapping(
            map.iter()
                .map(|(key, value)| (convert(key, pos), convert(value, pos)))
                .collect(),
        )),
        // タグ付きの集合。saphyr はタグの中のノードに位置を付けないので、外側の位置を使う
        YamlData::Tagged(tag, inner) => {
            let mut node = convert(inner, pos);
            node.offset = offset;
            node.tag.get_or_insert_with(|| Tag {
                handle: tag.handle.clone(),
                suffix: tag.suffix.clone(),
            });
            node
        }
        // 作っている途中のアンカーを指す循環した別名などで saphyr が値を作れなかったもの（BadValue）。
        // 値なしとは区別する（値なしにすると frontmatter の型の検査を素通りする。PR #108 のレビュー指摘。
        // `early_parse` は無効なので Value は作られない）
        _ => leaf(Kind::Unresolved),
    }
}

fn style_of(style: ScalarStyle) -> Style {
    match style {
        ScalarStyle::Plain => Style::Plain,
        ScalarStyle::SingleQuoted => Style::SingleQuoted,
        ScalarStyle::DoubleQuoted => Style::DoubleQuoted,
        ScalarStyle::Literal => Style::Literal,
        ScalarStyle::Folded => Style::Folded,
    }
}

/// バイト位置を 1 始まりの行・バイト基準の列にする。改行は LF・CR・CRLF（YAML 1.2 と同じ。
/// CRLF は 1 つの改行として数える）
///
/// ```
/// assert_eq!(shikuwasa::line_col("a: 1\nタイトル: x\n", 19), (2, 15));
/// assert_eq!(shikuwasa::line_col("a: 1\rb: 2\r\nc: 3", 12), (3, 2));
/// ```
pub fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let end = floor_char_boundary(text, offset);
    let bytes = text.as_bytes();
    let (mut line, mut line_start) = (1, 0);
    for i in 0..end {
        let breaks = match bytes[i] {
            b'\n' => true,
            // CRLF の CR は数えない（続く LF で数える）
            b'\r' => bytes.get(i + 1) != Some(&b'\n'),
            _ => false,
        };
        if breaks {
            line += 1;
            line_start = i + 1;
        }
    }
    (line, end - line_start + 1)
}

/// `offset` 以下でいちばん近い文字の境界（入力の長さを超えたら長さ）
fn floor_char_boundary(text: &str, offset: usize) -> usize {
    let mut i = offset.min(text.len());
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// YAML（JSON を含む）を JSON の値へ写す。マッピングのキーはスカラーの原文（`200:` は `"200"`）。
/// 有限でない小数（`.inf`・`.nan`）は JSON に無いので null。マッピングの記述順を保つ。
/// 文書が無ければ null。キーが配列やマッピングならエラー
///
/// ```
/// let value = shikuwasa::to_json("b: 1\na: [x, 2.5, ~]\n200: ok\n").unwrap();
/// assert_eq!(value, serde_json::json!({"b": 1, "a": ["x", 2.5, null], "200": "ok"}));
/// ```
#[cfg(feature = "json")]
pub fn to_json(text: &str) -> Result<serde_json::Value, Error> {
    match parse(text)? {
        None => Ok(serde_json::Value::Null),
        Some(root) => json_of(&root).map_err(|offset| {
            Error::at(
                text,
                offset,
                "マッピングのキーに配列やマッピングは使えません".to_string(),
            )
        }),
    }
}

/// ノードを JSON の値にする。キーが集合なら、その位置を返す
#[cfg(feature = "json")]
fn json_of(node: &Node) -> Result<serde_json::Value, usize> {
    use serde_json::Value;
    Ok(match &node.kind {
        Kind::Scalar(s) => match s.value {
            ScalarValue::Null => Value::Null,
            ScalarValue::Bool(b) => Value::Bool(b),
            ScalarValue::Int(v) => Value::from(v),
            ScalarValue::UInt(v) => Value::from(v),
            ScalarValue::Float(v) => {
                serde_json::Number::from_f64(v).map_or(Value::Null, Value::Number)
            }
            ScalarValue::Str => Value::String(s.text.clone()),
        },
        Kind::Sequence(items) => Value::Array(items.iter().map(json_of).collect::<Result<_, _>>()?),
        Kind::Mapping(entries) => {
            let mut object = serde_json::Map::new();
            for (key, value) in entries {
                let Kind::Scalar(k) = &key.kind else {
                    return Err(key.offset);
                };
                object.insert(k.text.clone(), json_of(value)?);
            }
            Value::Object(object)
        }
        // 値を作れなかった別名（setoka-core の yaml.rs のときから null にしている）
        Kind::Unresolved => Value::Null,
    })
}

#[cfg(test)]
mod tests;
