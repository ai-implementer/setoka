//! 文字列を受ける検査のテスト（Markdown から診断までの通しのテストは setoka-core の lint.rs にある）

use super::*;

#[test]
fn ルールの名前は_setoka_の診断の名前と同じ() {
    let names: Vec<&str> = Rule::ALL.iter().map(|r| r.name()).collect();
    assert_eq!(
        names,
        [
            "fullwidth-alphanumeric",
            "halfwidth-kana",
            "katakana-choon",
            "term-variant"
        ]
    );
}

#[test]
fn 全角英数字は連なりごとに半角を提案する() {
    let found = fullwidth_alphanumeric("前ＡＢｃ後１２");
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].range, 3..12);
    assert_eq!(found[0].found, "ＡＢｃ");
    assert_eq!(found[0].suggestion.as_deref(), Some("ABc"));
    assert_eq!(found[1].suggestion.as_deref(), Some("12"));
    assert!(
        fullwidth_alphanumeric("ABC と ＠").is_empty(),
        "全角の記号は対象外"
    );
}

#[test]
fn 半角カナは濁点と半濁点を合成する() {
    let found = halfwidth_kana("ｶﾞｲﾄﾞとﾎﾟｲﾝﾄとｳﾞｧ");
    let suggestions: Vec<_> = found
        .iter()
        .filter_map(|f| f.suggestion.as_deref())
        .collect();
    assert_eq!(suggestions, ["ガイド", "ポイント", "ヴァ"]);
    // 合成できない濁点は残す
    assert_eq!(halfwidth_kana("ｱﾞ")[0].suggestion.as_deref(), Some("ア゛"));
    assert_eq!(
        halfwidth_kana("ｻｰﾊﾞ")[0].message,
        "半角カナ「ｻｰﾊﾞ」は全角「サーバ」を推奨します"
    );
}

#[test]
fn 用語のゆれは正表記の中の出現を除く() {
    let terms = BTreeMap::from([(
        "サーバー".to_string(),
        vec!["サーバ".to_string(), String::new(), "サーバー".to_string()],
    )]);
    let found = term_variants("サーバーとサーバとサーバー", &terms);
    assert_eq!(found.len(), 1);
    assert_eq!(
        &"サーバーとサーバとサーバー"[found[0].range.clone()],
        "サーバ"
    );
    assert_eq!(found[0].suggestion.as_deref(), Some("サーバー"));
}

#[test]
fn 長音符のゆれは少数派を報告し多数派を候補にする() {
    let mut tally = ChoonTally::new();
    tally.add("ユーザーとユーザー", 1);
    tally.add("ユーザ", 2);
    let found = tally.finish();
    assert_eq!(found.len(), 1);
    let (at, f) = &found[0];
    assert_eq!(*at, 2);
    assert_eq!(f.range, 0..9);
    assert_eq!(f.suggestion.as_deref(), Some("ユーザー"));
    assert_eq!(
        f.message,
        "長音符ゆれ「ユーザ」が混在しています（ユーザ: 1 回 / ユーザー: 2 回。多数派への統一を推奨）"
    );
}

#[test]
fn 長音符のゆれが同数なら両方を報告して候補なし() {
    let mut tally = ChoonTally::new();
    tally.add("サーバとサーバー", "a");
    let found = tally.finish();
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|(_, f)| f.suggestion.is_none()));
}

#[test]
fn 短い語と表記が一種類の語は長音符のゆれにしない() {
    let mut tally = ChoonTally::new();
    // いちばん短い表記が 3 文字未満（カド / カード）
    tally.add("カドとカード", ());
    // 長音符を除いて 1 文字
    tally.add("ルとルー", ());
    // 表記が 1 種類
    tally.add("サーバーとサーバー", ());
    assert!(tally.finish().is_empty());
}
