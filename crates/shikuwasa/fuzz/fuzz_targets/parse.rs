//! 任意の入力で panic・hang・メモリの使いすぎが無いこと（不正な UTF-8 は前段で弾く）。
//! 読めた木と JSON への写しの位置が入力の中を指すことも見る

#![no_main]

use libfuzzer_sys::fuzz_target;

fn check(text: &str, node: &shikuwasa::Node) {
    assert!(node.offset <= text.len(), "位置が入力の外");
    assert!(text.is_char_boundary(node.offset), "位置が文字の途中");
    let _ = shikuwasa::line_col(text, node.offset);
    match &node.kind {
        shikuwasa::Kind::Scalar(_) | shikuwasa::Kind::Unresolved => {}
        shikuwasa::Kind::Sequence(items) => items.iter().for_each(|n| check(text, n)),
        shikuwasa::Kind::Mapping(entries) => entries.iter().for_each(|(k, v)| {
            check(text, k);
            check(text, v);
        }),
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    match shikuwasa::parse(text) {
        Ok(Some(root)) => check(text, &root),
        Ok(None) => {}
        Err(e) => {
            assert!(e.offset <= text.len(), "エラーの位置が入力の外");
            assert!(e.line >= 1 && e.col >= 1);
        }
    }
    let _ = shikuwasa::to_json(text);
});
