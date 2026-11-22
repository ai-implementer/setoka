//! スカラーの型の解釈（YAML 1.2 の core schema。境界は serde_yaml_ng に合わせてある）

/// スカラーを解釈した値。文字列の中身は [`crate::Scalar::text`] にある
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScalarValue {
    /// `~`・`null`・空
    Null,
    Bool(bool),
    Int(i64),
    /// i64 に収まらない正の整数
    UInt(u64),
    /// 有限でない値（`.inf`・`.nan`）も含む
    Float(f64),
    /// 文字列（引用符・ブロック・`!!str`・ほかの型に読めない書き方）
    Str,
}

/// 書き方とタグからスカラーを解釈する。明示のコアタグを引用符より先に見る
/// （`!!bool "true"` は真偽値・`!!str 1` は文字列。serde_yaml_ng と同じ。タグに合わない値は
/// 読み込みがエラーにしている）。タグが無ければ、引用符・ブロック（`|` / `>`）は文字列。
/// `core_tag` はコアタグ（`tag:yaml.org,2002:`）の名前（`bool` など）
pub(crate) fn resolve(text: &str, plain: bool, core_tag: Option<&str>) -> ScalarValue {
    match core_tag {
        Some("str") => ScalarValue::Str,
        Some(suffix) => match resolve_tagged(text, suffix) {
            Some(resolved) => resolved,
            None => untagged(text, plain),
        },
        None => untagged(text, plain),
    }
}

fn untagged(text: &str, plain: bool) -> ScalarValue {
    if plain {
        resolve_plain(text)
    } else {
        ScalarValue::Str
    }
}

/// 型を指定するコアタグ（`!!bool` / `!!int` / `!!float` / `!!null`）が付いた値の解釈。
/// 引用符の有無に依らず原文を解釈する。ほかのタグは `None`（タグで型を決めない）
pub(crate) fn resolve_tagged(text: &str, suffix: &str) -> Option<ScalarValue> {
    // ブロック（`!!int |`）の値は末尾に改行が付くので外してから読む
    let resolved = resolve_plain(text.trim_end_matches(['\n', '\r']));
    match suffix {
        "bool" | "int" | "null" => Some(resolved),
        // `!!float 1` は小数の 1.0
        "float" => Some(match resolved {
            ScalarValue::Int(v) => ScalarValue::Float(v as f64),
            ScalarValue::UInt(v) => ScalarValue::Float(v as f64),
            other => other,
        }),
        _ => None,
    }
}

/// 引用符もタグも無いスカラー（plain scalar）として解釈する。
///
/// - 値なし: `""`・`~`・`null`・`Null`・`NULL`
/// - 真偽値: `true`・`True`・`TRUE`・`false`・`False`・`FALSE`（YAML 1.1 の `yes` / `on` は文字列）
/// - 整数: 10 進・`0x`・`0o`・`0b`（符号付き）。先頭 0 の 10 進数（`010`）と `_` 区切りは文字列。
///   i64 に収まらない正の数は [`ScalarValue::UInt`]、u64 にも収まらない 10 進数は小数
/// - 小数: `1.5`・`.5`・`5.`・`1e3`・`.inf`・`.nan`。有限でない結果になる数字（`1e400`）は文字列
///
/// ```
/// use shikuwasa::{ScalarValue, resolve_plain};
/// assert_eq!(resolve_plain("0x1F"), ScalarValue::Int(31));
/// assert_eq!(resolve_plain("010"), ScalarValue::Str);
/// assert_eq!(resolve_plain("yes"), ScalarValue::Str);
/// ```
pub fn resolve_plain(text: &str) -> ScalarValue {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => ScalarValue::Null,
        "true" | "True" | "TRUE" => ScalarValue::Bool(true),
        "false" | "False" | "FALSE" => ScalarValue::Bool(false),
        _ => int_of(text)
            .or_else(|| float_of(text))
            .unwrap_or(ScalarValue::Str),
    }
}

/// 符号の後が 10 進・`0x`・`0o`・`0b` の整数か。先頭 0 の 10 進数（`010`）は YAML 1.1 の
/// 8 進数と紛らわしいので文字列にする（serde_yaml_ng と同じ）。範囲外の 10 進数は小数
fn int_of(text: &str) -> Option<ScalarValue> {
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let (radix, body) = if let Some(hex) = digits.strip_prefix("0x") {
        (16, hex)
    } else if let Some(oct) = digits.strip_prefix("0o") {
        (8, oct)
    } else if let Some(bin) = digits.strip_prefix("0b") {
        (2, bin)
    } else if digits.len() > 1 && digits.starts_with('0') {
        return None;
    } else {
        (10, digits)
    };
    if body.is_empty() || !body.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    let magnitude = u64::from_str_radix(body, radix).ok()?;
    if negative {
        // i64::MIN の絶対値は i64 に収まらないので 0 から引く
        0i64.checked_sub_unsigned(magnitude).map(ScalarValue::Int)
    } else {
        Some(i64::try_from(magnitude).map_or(ScalarValue::UInt(magnitude), ScalarValue::Int))
    }
}

/// `1.5`・`.5`・`5.`・`1e3`・`.inf`・`.nan` の形か。`.` も指数も無い数字だけの並びは
/// 整数の側で扱う（先頭 0 の `010` を小数として読まない）。有限でない結果（`1e400`）は文字列
fn float_of(text: &str) -> Option<ScalarValue> {
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    if matches!(unsigned, ".inf" | ".Inf" | ".INF") {
        let sign = if text.starts_with('-') { -1.0 } else { 1.0 };
        return Some(ScalarValue::Float(sign * f64::INFINITY));
    }
    if matches!(text, ".nan" | ".NaN" | ".NAN") {
        return Some(ScalarValue::Float(f64::NAN));
    }
    let bytes = unsigned.as_bytes();
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let mut i = digits(0);
    let int_digits = i;
    let mut point = false;
    let mut frac_digits = 0;
    if bytes.get(i) == Some(&b'.') {
        point = true;
        frac_digits = digits(i + 1);
        i += 1 + frac_digits;
    }
    if int_digits + frac_digits == 0 {
        return None;
    }
    let mut exponent = false;
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let n = digits(i);
        if n == 0 {
            return None;
        }
        i += n;
        exponent = true;
    }
    if i != bytes.len() || !(point || exponent) {
        return None;
    }
    text.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(ScalarValue::Float)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn スカラーの解釈は置き換え前と同じ境界にする() {
        for (text, expected) in [
            ("~", ScalarValue::Null),
            ("", ScalarValue::Null),
            ("NULL", ScalarValue::Null),
            ("nULL", ScalarValue::Str),
            ("True", ScalarValue::Bool(true)),
            ("tRUE", ScalarValue::Str),
            ("yes", ScalarValue::Str),
            ("0", ScalarValue::Int(0)),
            ("-0", ScalarValue::Int(0)),
            ("+5", ScalarValue::Int(5)),
            ("010", ScalarValue::Str),
            ("00", ScalarValue::Str),
            ("1_000", ScalarValue::Str),
            ("0x1F", ScalarValue::Int(31)),
            ("0X1F", ScalarValue::Str),
            ("-0x10", ScalarValue::Int(-16)),
            ("0o17", ScalarValue::Int(15)),
            ("-0o7", ScalarValue::Int(-7)),
            ("0b101", ScalarValue::Int(5)),
            ("0x", ScalarValue::Str),
            ("-9223372036854775808", ScalarValue::Int(i64::MIN)),
            (
                "9223372036854775808",
                ScalarValue::UInt(9_223_372_036_854_775_808),
            ),
            ("18446744073709551615", ScalarValue::UInt(u64::MAX)),
            ("1.0", ScalarValue::Float(1.0)),
            (".5", ScalarValue::Float(0.5)),
            ("5.", ScalarValue::Float(5.0)),
            ("1e3", ScalarValue::Float(1000.0)),
            ("-.Inf", ScalarValue::Float(f64::NEG_INFINITY)),
            ("1e400", ScalarValue::Str),
            ("1e", ScalarValue::Str),
            (".", ScalarValue::Str),
            ("inf", ScalarValue::Str),
            ("3.0.1", ScalarValue::Str),
            ("1:30", ScalarValue::Str),
            ("2024-01-01", ScalarValue::Str),
        ] {
            assert_eq!(resolve_plain(text), expected, "{text:?}");
        }
        assert!(matches!(resolve_plain(".nan"), ScalarValue::Float(v) if v.is_nan()));
    }
}
