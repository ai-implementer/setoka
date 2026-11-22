//! 読み込みのテスト（setoka-core の yaml.rs から移したもの。JSON への写しは feature `json` のとき）

use super::*;
use crate::load::DUPLICATE_KEY;

fn root(text: &str) -> Node {
    parse(text).unwrap().unwrap()
}

fn entries(node: &Node) -> &[(Node, Node)] {
    node.as_mapping().expect("マッピング")
}

#[test]
fn 引用符とブロックと_str_タグは文字列() {
    let root = root("a: \"1\"\nb: '~'\nc: |\n  true\nd: !!str 12\ne: !custom 12\n");
    let values: Vec<ScalarValue> = entries(&root)
        .iter()
        .map(|(_, v)| v.as_scalar().expect("スカラー").value)
        .collect();
    assert_eq!(
        values,
        [
            ScalarValue::Str,
            ScalarValue::Str,
            ScalarValue::Str,
            ScalarValue::Str,
            ScalarValue::Int(12)
        ]
    );
    let styles: Vec<Style> = entries(&root)
        .iter()
        .map(|(_, v)| v.as_scalar().unwrap().style)
        .collect();
    assert_eq!(
        styles,
        [
            Style::DoubleQuoted,
            Style::SingleQuoted,
            Style::Literal,
            Style::Plain,
            Style::Plain
        ]
    );
    // タグは原文の情報として残す
    let d = &entries(&root)[3].1;
    assert!(
        d.tag
            .as_ref()
            .is_some_and(|t| t.is_core() && t.suffix == "str")
    );
    let e = &entries(&root)[4].1;
    assert!(
        e.tag
            .as_ref()
            .is_some_and(|t| !t.is_core() && t.suffix == "custom")
    );
}

#[test]
fn 文書が無ければ_none() {
    assert_eq!(parse("").unwrap(), None);
    assert_eq!(parse("# コメントだけ\n").unwrap(), None);
    assert!(root("~\n").is_null());
}

#[test]
fn 別名は展開し入れ子で膨らむ文書は上限で止める() {
    let root = root("a: &x [1, 2]\nb: *x\n");
    assert_eq!(entries(&root)[0].1.kind, entries(&root)[1].1.kind);
    // 10 個ずつ 6 段 = 100 万ノード
    let mut text = String::from("a0: &a0 [x,x,x,x,x,x,x,x,x,x]\n");
    for i in 1..6 {
        let p = i - 1;
        let refs = vec![format!("*a{p}"); 10].join(",");
        text.push_str(&format!("a{i}: &a{i} [{refs}]\n"));
    }
    let e = parse(&text).unwrap_err();
    assert!(e.message.contains("多すぎます"), "{e}");
    assert_eq!(e.line, 6, "上限を超えた別名の行: {e}");
}

/// saphyr の loader の上限（既定 10 万ノード）より先に、こちらの上限（20 万）で判定する
/// （setoka の PR #79 のレビュー指摘。saphyr の既定のままだと、下の 2 つ目が英語の構文エラーになる）
#[test]
fn 別名の展開は上限ちょうどまで通し超えたら日本語のエラーにする() {
    // 10 個ずつ 5 段 = 別名の展開 123,440 ノード（saphyr の既定の上限を超え、こちらの上限以下）
    let mut text = String::from("a0: &a0 [x,x,x,x,x,x,x,x,x,x]\n");
    for i in 1..5 {
        let p = i - 1;
        let refs = vec![format!("*a{p}"); 10].join(",");
        text.push_str(&format!("a{i}: &a{i} [{refs}]\n"));
    }
    assert!(parse(&text).is_ok(), "{:?}", parse(&text).err());
    // 別名 1 つで展開するのが、配列 1 つと要素 n 個 = n + 1 ノード
    let expand = |n: usize| format!("a: &x [{}]\nb: *x\n", vec!["1"; n].join(","));
    assert!(
        parse(&expand(MAX_ALIAS_NODES - 1)).is_ok(),
        "上限ちょうどは通す"
    );
    let e = parse(&expand(MAX_ALIAS_NODES)).unwrap_err();
    assert!(
        e.message.contains("多すぎます"),
        "上限を 1 つ超えたら日本語のエラー: {e}"
    );
    assert_eq!(e.line, 2, "{e}");
}

#[test]
fn エラーは日本語で位置は行とバイト基準の列() {
    let e = parse("タイトル: x\ntitle: a\ntitle: b\n").unwrap_err();
    assert_eq!((e.line, e.col), (3, 1));
    assert_eq!(e.offset, "タイトル: x\ntitle: a\n".len());
    assert!(e.message.contains("重複"), "{e}");
    assert_eq!(e.to_string(), format!("{}（3 行 1 列）", e.message));
    // 列はバイト基準（「タイトル」は 12 バイト）
    let e = parse("タイトル: [x\n").unwrap_err();
    assert!(e.message.contains("閉じていません"), "{e}");
    let e = parse("title:\tx\n").unwrap_err();
    assert!(e.message.contains("タブ"), "{e}");
    let e = parse("a: 1\n---\nb: 2\n").unwrap_err();
    assert_eq!(e.line, 3, "2 つ目の文書の位置: {e}");
    assert!(e.message.contains("複数"), "{e}");
}

/// setoka の PR #55 のレビュー指摘: saphyr は `early_parse` 無効だとスタイルとタグも比べるので、
/// 引用符だけが違う同名のキーを別のキーとして通し、後の値が黙って勝っていた
#[test]
fn 引用符やタグだけが違う同名のキーも重複にする() {
    for (text, line) in [
        ("a: 1\n\"a\": 2\n", 2),
        ("a: 1\n'a': 2\n", 2),
        ("200: x\n\"200\": y\n", 2),
        ("a: 1\n!!str a: 2\n", 2),
        ("x:\n  a: 1\n  b: 2\n  \"a\": 3\n", 4),
        ("- {a: 1, \"a\": 2}\n", 1),
    ] {
        let e = parse(text).unwrap_err();
        assert_eq!(e.message, DUPLICATE_KEY, "{text:?}");
        assert_eq!(e.line, line, "{text:?}");
    }
    // 別の階層の同名キーは重複ではない
    assert!(parse("a: 1\nb:\n  a: 2\n").is_ok());
}

/// setoka の PR #55 のレビュー指摘: 引用符を先に見て `!!bool "true"` を文字列にしていた
/// （OpenAPI の `nullable: !!bool "true"` の「null 許容」が消えた）
#[test]
fn 明示のコアタグは引用符より先に見る() {
    let root = root(
        "b: !!bool \"true\"\nf: !!bool 'false'\ni: !!int \"42\"\nh: !!int \"0x10\"\nfl: !!float \"1\"\nn: !!null \"~\"\ns: !!str 1\nblock: !!int |\n  42\ncustom: !x \"1\"\n",
    );
    let values: Vec<ScalarValue> = entries(&root)
        .iter()
        .map(|(_, v)| v.as_scalar().unwrap().value)
        .collect();
    assert_eq!(
        values,
        [
            ScalarValue::Bool(true),
            ScalarValue::Bool(false),
            ScalarValue::Int(42),
            ScalarValue::Int(16),
            ScalarValue::Float(1.0),
            ScalarValue::Null,
            ScalarValue::Str,
            ScalarValue::Int(42),
            ScalarValue::Str,
        ]
    );
    // タグに合わない値はエラー（serde_yaml_ng と同じ）
    for (text, expected) in [
        (
            "v: !!int \"x\"\n",
            "`!!int` のタグが付いた `x` は整数として読めません",
        ),
        (
            "v: !!bool yes\n",
            "`!!bool` のタグが付いた `yes` は真偽値として読めません",
        ),
        (
            "v: !!int 010\n",
            "`!!int` のタグが付いた `010` は整数として読めません",
        ),
        (
            "v: !!null x\n",
            "`!!null` のタグが付いた `x` は値なし（null）として読めません",
        ),
    ] {
        let e = parse(text).unwrap_err();
        assert_eq!(e.message, expected, "{text:?}");
        assert_eq!(e.line, 1, "値の位置: {text:?}");
    }
}

#[test]
fn 位置は文字数からバイトへ直す() {
    let text = "タイトル: 値\nlist:\n  - あ\n  - い\n";
    let root = root(text);
    let (key, value) = &entries(&root)[0];
    assert_eq!(key.offset, 0);
    assert_eq!(&text[value.offset..], "値\nlist:\n  - あ\n  - い\n");
    assert_eq!(line_col(text, value.offset), (1, 15));
    let items = entries(&root)[1].1.as_sequence().unwrap();
    assert_eq!(&text[items[1].offset..], "い\n");
    // ASCII だけの入力も同じ
    let ascii = "a: 1\nb: [x, y]\n";
    let root = self::root(ascii);
    let items = entries(&root)[1].1.as_sequence().unwrap();
    assert_eq!(&ascii[items[1].offset..], "y]\n");
}

#[test]
fn タグ付きの集合は外側の位置とタグを持つ() {
    let text = "a: !point\n  x: 1\nb: !list [1]\nc: !!map {x: 1}\n";
    let root = root(text);
    let (_, a) = &entries(&root)[0];
    assert!(a.as_mapping().is_some());
    assert_eq!(a.tag.as_ref().map(|t| t.suffix.as_str()), Some("point"));
    assert_eq!(line_col(text, a.offset), (1, 4));
    let (_, b) = &entries(&root)[1];
    assert!(b.as_sequence().is_some());
    assert_eq!(b.tag.as_ref().map(|t| t.suffix.as_str()), Some("list"));
    assert_eq!(line_col(text, b.offset), (3, 4));
    // 集合に付けたコアタグは saphyr が残さない
    let (_, c) = &entries(&root)[2];
    assert!(c.as_mapping().is_some() && c.tag.is_none());
}

#[test]
fn 行と列は文字の途中を指しても文字の頭で数える() {
    // 「あ」は 3 バイト。2 バイト目を指しても落ちない
    assert_eq!(line_col("あい", 1), (1, 1));
    assert_eq!(line_col("あい", 3), (1, 4));
    assert_eq!(line_col("a\nb", 99), (2, 2));
}

#[cfg(feature = "json")]
mod json {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_へ写すとキーは原文で順序を保つ() {
        let value = to_json("b: 1\na: [x, 2.5, ~, true]\n200: {z: .inf}\n").unwrap();
        assert_eq!(
            value,
            json!({"b": 1, "a": ["x", 2.5, null, true], "200": {"z": null}})
        );
        let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["b", "a", "200"], "記述順");
        // JSON もそのまま読める
        assert_eq!(
            to_json(r#"{"a": "あ", "b": [1, -2]}"#).unwrap(),
            json!({"a": "あ", "b": [1, -2]})
        );
        assert_eq!(to_json("").unwrap(), json!(null));
        assert_eq!(to_json("# コメントだけ\n").unwrap(), json!(null));
    }

    #[test]
    fn 別名は値を写す() {
        assert_eq!(
            to_json("a: &x [1, 2]\nb: *x\n").unwrap(),
            json!({"a": [1, 2], "b": [1, 2]})
        );
    }

    #[test]
    fn コアタグは型を決める() {
        assert_eq!(
            to_json(
                "b: !!bool \"true\"\nf: !!bool 'false'\ni: !!int \"42\"\nh: !!int \"0x10\"\nfl: !!float \"1\"\nn: !!null \"~\"\ns: !!str 1\nblock: !!int |\n  42\ncustom: !x \"1\"\n"
            )
            .unwrap(),
            json!({"b": true, "f": false, "i": 42, "h": 16, "fl": 1.0, "n": null, "s": "1", "block": 42, "custom": "1"})
        );
    }

    #[test]
    fn キーが集合ならエラー() {
        let e = to_json("x: 1\n[a]: 1\n").unwrap_err();
        assert_eq!(e.message, "マッピングのキーに配列やマッピングは使えません");
        assert_eq!((e.line, e.col), (2, 1));
    }
}
