//! # hebesu
//!
//! 日本語の文章の**表記ゆれ**を見つける。どれも文字列を受け取り、見つけたゆれをバイトの範囲・変換の候補・
//! 日本語の説明付きで返す（Markdown などの構造は読まない。本文のテキストを取り出すのは使う側）。
//!
//! | ルール | 見つけるもの | 関数 |
//! | --- | --- | --- |
//! | `fullwidth-alphanumeric` | 全角英数字（`ＡＢＣ１２３`）→ 半角を提案 | [`fullwidth_alphanumeric`] |
//! | `halfwidth-kana` | 半角カナ（`ｻｰﾊﾞ`）→ 全角を提案（濁点・半濁点は合成する） | [`halfwidth_kana`] |
//! | `term-variant` | 用語辞書のゆれ（`サーバ` → `サーバー`）。正表記の一部としての出現は除く | [`term_variants`] |
//! | `katakana-choon` | 長音符のゆれの混在（`サーバ` と `サーバー`）。複数の文章を集計し、少数派を報告する | [`ChoonTally`] |
//!
//! ルールの名前（[`Rule::name`]）は setoka の診断の名前と同じ。説明の文言は日本語。
//!
//! ```
//! let found = hebesu::fullwidth_alphanumeric("Ｒｕｓｔで書く");
//! assert_eq!(found[0].range, 0..12);
//! assert_eq!(found[0].suggestion.as_deref(), Some("Rust"));
//!
//! // 長音符のゆれは文章の集まりで多数派を決める（位置の印は使う側が決める型）
//! let mut tally = hebesu::ChoonTally::new();
//! tally.add("サーバーを起動する", "a.md");
//! tally.add("サーバーを止める", "b.md");
//! tally.add("サーバを再起動する", "c.md");
//! let found = tally.finish();
//! assert_eq!(found.len(), 1);
//! assert_eq!(found[0].0, "c.md");
//! assert_eq!(found[0].1.suggestion.as_deref(), Some("サーバー"));
//! ```

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::ops::Range;

/// ルール
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rule {
    /// 全角英数字
    FullwidthAlphanumeric,
    /// 半角カナ
    HalfwidthKana,
    /// 長音符のゆれの混在
    KatakanaChoon,
    /// 用語辞書のゆれ
    TermVariant,
}

impl Rule {
    /// すべてのルール
    pub const ALL: [Rule; 4] = [
        Rule::FullwidthAlphanumeric,
        Rule::HalfwidthKana,
        Rule::KatakanaChoon,
        Rule::TermVariant,
    ];

    /// ルールの名前（setoka の診断の名前と同じ）
    pub fn name(self) -> &'static str {
        match self {
            Rule::FullwidthAlphanumeric => "fullwidth-alphanumeric",
            Rule::HalfwidthKana => "halfwidth-kana",
            Rule::KatakanaChoon => "katakana-choon",
            Rule::TermVariant => "term-variant",
        }
    }
}

/// 見つけたゆれ 1 件
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Finding {
    pub rule: Rule,
    /// 渡した文字列の中のバイトの範囲。**複数のルール・辞書の項目の結果は範囲が重なることがある**
    /// （`DBサーバ` に対する `DB` の項目と `サーバ` → `サーバー` の項目など）。自動修正では、重なる・交差する
    /// 候補を選別してから（後ろから見て、置き換えた範囲と交差するものを捨てる等）、残った候補を後ろから順に
    /// 置き換える（そのまま全部を後ろから置き換えると `DBー` のように壊れる）
    pub range: Range<usize>,
    /// 見つけた表記（`range` の部分）
    pub found: String,
    /// 置き換えの候補。決められないとき（長音符のゆれで表記の数が同じ）は `None`
    pub suggestion: Option<String>,
    /// 日本語の説明
    pub message: String,
}

/// 全角英数字（`０-９`・`Ａ-Ｚ`・`ａ-ｚ`）の連なりを見つけ、半角を提案する
///
/// ```
/// let found = hebesu::fullwidth_alphanumeric("ＡＢＣと１２３");
/// assert_eq!(found.len(), 2);
/// assert_eq!(found[1].message, "全角英数字「１２３」は半角「123」を推奨します");
/// ```
pub fn fullwidth_alphanumeric(text: &str) -> Vec<Finding> {
    let is_fullwidth_alnum = |c: char| matches!(c, '\u{FF10}'..='\u{FF19}' | '\u{FF21}'..='\u{FF3A}' | '\u{FF41}'..='\u{FF5A}');
    runs(text, is_fullwidth_alnum)
        .map(|(range, run)| {
            // 全角英数字 → 半角はコード点 −0xFEE0 の単純対応
            let suggestion: String = run
                .chars()
                .map(|c| char::from_u32(c as u32 - 0xFEE0).unwrap_or(c))
                .collect();
            Finding {
                rule: Rule::FullwidthAlphanumeric,
                message: format!("全角英数字「{run}」は半角「{suggestion}」を推奨します"),
                range,
                found: run.to_string(),
                suggestion: Some(suggestion),
            }
        })
        .collect()
}

/// 半角カナ（U+FF61〜FF9F）の連なりを見つけ、全角を提案する（濁点・半濁点は前の文字と合成する）
///
/// ```
/// let found = hebesu::halfwidth_kana("ｻｰﾊﾞとﾊﾟｽ");
/// assert_eq!(found[0].suggestion.as_deref(), Some("サーバ"));
/// assert_eq!(found[1].suggestion.as_deref(), Some("パス"));
/// ```
pub fn halfwidth_kana(text: &str) -> Vec<Finding> {
    let is_halfwidth_kana = |c: char| matches!(c, '\u{FF61}'..='\u{FF9F}');
    runs(text, is_halfwidth_kana)
        .map(|(range, run)| {
            let suggestion = to_fullwidth_kana(run);
            Finding {
                rule: Rule::HalfwidthKana,
                message: format!("半角カナ「{run}」は全角「{suggestion}」を推奨します"),
                range,
                found: run.to_string(),
                suggestion: Some(suggestion),
            }
        })
        .collect()
}

/// 用語辞書（正表記 → ゆれ表記の一覧）のゆれ表記を見つけ、正表記を提案する。
/// 正表記の一部としての出現（正「サーバー」の中の「サーバ」）は除く。空のゆれ表記と正表記と同じゆれ表記は無視する。
/// 並びは辞書の正表記の順 → ゆれ表記の順 → 出現の順
///
/// ```
/// use std::collections::BTreeMap;
/// let terms = BTreeMap::from([("サーバー".to_string(), vec!["サーバ".to_string()])]);
/// let found = hebesu::term_variants("サーバーとサーバ", &terms);
/// assert_eq!(found.len(), 1);
/// assert_eq!(found[0].range, 15..24);
/// assert_eq!(found[0].message, "「サーバ」は「サーバー」に統一してください");
/// ```
pub fn term_variants(text: &str, terms: &BTreeMap<String, Vec<String>>) -> Vec<Finding> {
    let mut out = Vec::new();
    for (canonical, variants) in terms {
        // 正表記の出現区間（バイト範囲）。この中の variant マッチは除外する
        let canon_ranges: Vec<(usize, usize)> = text
            .match_indices(canonical.as_str())
            .map(|(i, m)| (i, i + m.len()))
            .collect();
        for variant in variants {
            if variant.is_empty() || variant == canonical {
                continue;
            }
            for (i, m) in text.match_indices(variant.as_str()) {
                let end = i + m.len();
                if canon_ranges.iter().any(|&(s, e)| s <= i && end <= e) {
                    continue;
                }
                out.push(Finding {
                    rule: Rule::TermVariant,
                    range: i..end,
                    found: variant.clone(),
                    suggestion: Some(canonical.clone()),
                    message: format!("「{variant}」は「{canonical}」に統一してください"),
                });
            }
        }
    }
    out
}

/// 長音符のゆれの集計。文章を位置の印（使う側が決める型。ファイル名やテキストの位置など）と一緒に
/// [`add`](Self::add) で足していき、[`finish`](Self::finish) で少数派の出現を受け取る。
///
/// 長音符（ー）を除いた形が同じカタカナ語を同じ語とみなし、表記が複数あれば**少数派の出現**を報告する
/// （単独の多数派を置き換えの候補にする。いちばん多い表記が複数あれば、どの表記も報告して候補は無し）。
/// 長音符を除いて 2 文字未満の語と、いちばん短い表記が 3 文字未満の語（「カド」と「カード」のような
/// 別の語の偶然の一致）は対象外
#[derive(Debug, Clone)]
pub struct ChoonTally<K> {
    /// 長音符を除いた形 → 表記ごとの出現（BTreeMap で決定的な順にする）
    groups: BTreeMap<String, Variants<K>>,
}

/// 表記 → 出現（位置の印と、文章の中のバイトの範囲）
type Variants<K> = BTreeMap<String, Vec<(K, Range<usize>)>>;

impl<K> Default for ChoonTally<K> {
    fn default() -> Self {
        Self {
            groups: BTreeMap::new(),
        }
    }
}

impl<K: Clone> ChoonTally<K> {
    pub fn new() -> Self {
        Self::default()
    }

    /// 文章 `text` のカタカナ語を数える。出現は `at` と `text` の中のバイトの範囲で覚える
    pub fn add(&mut self, text: &str, at: K) {
        let is_katakana = |c: char| matches!(c, '\u{30A1}'..='\u{30FA}' | 'ー');
        for (range, word) in runs(text, is_katakana) {
            // ー のみ・実質 1 文字の連なりは語ではないので除く
            let key: String = word.chars().filter(|&c| c != 'ー').collect();
            if key.chars().count() < 2 {
                continue;
            }
            self.groups
                .entry(key)
                .or_default()
                .entry(word.to_string())
                .or_default()
                .push((at.clone(), range));
        }
    }

    /// 少数派の出現を返す。並びは語（長音符を除いた形）の順 → 表記の順 → 足した順
    pub fn finish(self) -> Vec<(K, Finding)> {
        let mut out = Vec::new();
        for variants in self.groups.into_values() {
            if variants.len() < 2 {
                continue; // 表記が 1 種類なら混在なし
            }
            // 最短の表記が 3 文字未満なら別語の偶然一致とみなして除外（カド/カード等）
            let min_surface = variants
                .keys()
                .map(|w| w.chars().count())
                .min()
                .unwrap_or(0);
            if min_surface < 3 {
                continue;
            }
            let max_count = variants.values().map(Vec::len).max().unwrap_or(0);
            // 単独多数派の表記（同数タイなら None = 正解が決められないので候補なし）
            let majority: Option<String> = {
                let mut top = variants.iter().filter(|(_, o)| o.len() == max_count);
                match (top.next(), top.next()) {
                    (Some((w, _)), None) => Some(w.clone()),
                    _ => None,
                }
            };
            let summary = variants
                .iter()
                .map(|(w, occs)| format!("{w}: {} 回", occs.len()))
                .collect::<Vec<_>>()
                .join(" / ");
            for (word, occs) in variants {
                if majority.as_ref() == Some(&word) {
                    continue; // 単独多数派は正とみなして報告しない
                }
                for (at, range) in occs {
                    out.push((
                        at,
                        Finding {
                            rule: Rule::KatakanaChoon,
                            range,
                            found: word.clone(),
                            suggestion: majority.clone(),
                            message: format!(
                                "長音符ゆれ「{word}」が混在しています（{summary}。多数派への統一を推奨）"
                            ),
                        },
                    ));
                }
            }
        }
        out
    }
}

/// 述語を満たす文字の最長の連なりを（バイトの範囲, 部分文字列）で列挙する
fn runs(text: &str, pred: impl Fn(char) -> bool) -> impl Iterator<Item = (Range<usize>, &str)> {
    let mut found = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        if pred(c) {
            start.get_or_insert(i);
        } else if let Some(s) = start.take() {
            found.push((s..i, &text[s..i]));
        }
    }
    if let Some(s) = start {
        found.push((s..text.len(), &text[s..]));
    }
    found.into_iter()
}

/// 半角カナ（U+FF61〜FF9F）→ 全角の変換候補。濁点/半濁点は可能なら合成する
fn to_fullwidth_kana(run: &str) -> String {
    // U+FF61 起点の並び順どおりの対応表
    const TABLE: &[char] = &[
        '。', '「', '」', '、', '・', 'ヲ', 'ァ', 'ィ', 'ゥ', 'ェ', 'ォ', 'ャ', 'ュ', 'ョ', 'ッ',
        'ー', 'ア', 'イ', 'ウ', 'エ', 'オ', 'カ', 'キ', 'ク', 'ケ', 'コ', 'サ', 'シ', 'ス', 'セ',
        'ソ', 'タ', 'チ', 'ツ', 'テ', 'ト', 'ナ', 'ニ', 'ヌ', 'ネ', 'ノ', 'ハ', 'ヒ', 'フ', 'ヘ',
        'ホ', 'マ', 'ミ', 'ム', 'メ', 'モ', 'ヤ', 'ユ', 'ヨ', 'ラ', 'リ', 'ル', 'レ', 'ロ', 'ワ',
        'ン', '゛', '゜',
    ];
    let mut out = String::new();
    for c in run.chars() {
        let mapped = TABLE
            .get((c as usize).wrapping_sub(0xFF61))
            .copied()
            .unwrap_or(c);
        match mapped {
            // 濁点: カ〜ト・ハ〜ホは +1 で濁音、ウはヴ
            '゛' => match out.pop() {
                Some(prev @ ('カ'..='ト' | 'ハ'..='ホ')) => {
                    out.push(char::from_u32(prev as u32 + 1).expect("カタカナ濁音"));
                }
                Some('ウ') => out.push('ヴ'),
                Some(prev) => {
                    out.push(prev);
                    out.push('゛');
                }
                None => out.push('゛'),
            },
            // 半濁点: ハ行は +2 で半濁音
            '゜' => match out.pop() {
                Some(prev @ 'ハ'..='ホ') => {
                    out.push(char::from_u32(prev as u32 + 2).expect("カタカナ半濁音"));
                }
                Some(prev) => {
                    out.push(prev);
                    out.push('゜');
                }
                None => out.push('゜'),
            },
            m => out.push(m),
        }
    }
    out
}

#[cfg(test)]
mod tests;
