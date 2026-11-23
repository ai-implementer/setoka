//! # kunenbo
//!
//! 出力ディレクトリへの**安全な書き込み**。静的サイトの生成器のように、利用者が書いた設定や前回のビルドの
//! 記録からパスを組み立てて、決まったディレクトリの下へ書くプログラム向け。v0.22 Phase 106 で setoka-core の
//! `output.rs` から切り出した。
//!
//! - **相対パスの検証**（[`resolve_output_rel`]）— 絶対パス・`..`・`.`・空のセグメント・`\` を断る。
//!   `Path::join` は絶対パスを渡すと左辺を捨て、`.` はファイルシステムが吸収するので、文字列の比較では防げない
//! - **経路のシンボリックリンクの検査**（[`ensure_no_symlink_under`]・[`ensure_symlink_free`]）— 基点自身から
//!   対象までの各要素にリンクがあれば、向き先に関わらず断る。基点の祖先は見ない（macOS の
//!   `/tmp -> private/tmp` のように、基点へ至る経路がリンクなのは普通のことなので）
//! - **書き込み**（[`write_under`]・[`write_atomic_under`]）— 上の 2 つを通し、親ディレクトリを作ってから書く。
//!   [`write_under`] は内容が同じなら書かない（更新時刻を変えない）。[`write_atomic_under`] は一時ファイルに
//!   書いてから置き換える
//! - **作成と削除**（[`create_dir_all_under`]・[`remove_dir_all_under`]・[`remove_file_under`]）— 同じ検査を
//!   通してから作る・消す
//! - **残ったファイルの掃除**（[`remove_orphans`]・[`list_files_under`]）— 前回書いて今回書かなかったファイルを
//!   消し、空になったディレクトリを剪定する。大文字小文字を区別しない FS で、改名した今回の出力を消さない
//!
//! 出力先へ書くコードは、`root.join(rel)` と `std::fs::write` を直に書かずにこの crate の関数を通す。
//! 文言（エラー）は日本語。
//!
//! ```
//! use kunenbo::WriteOutcome;
//!
//! let dir = tempfile::tempdir()?;
//! let dist = dir.path().join("dist");
//! let (path, outcome) = kunenbo::write_under(&dist, "guide/index.html", b"<h1>guide</h1>")?;
//! assert_eq!(outcome, WriteOutcome::Written);
//! // 同じ内容なら書かない
//! assert_eq!(kunenbo::write_under(&dist, "guide/index.html", b"<h1>guide</h1>")?.1, WriteOutcome::Unchanged);
//! // 出力先の外を指す相対パスは断る
//! assert!(kunenbo::write_under(&dist, "../outside.html", b"x").is_err());
//! # assert!(path.ends_with("guide/index.html"));
//! # Ok::<(), std::io::Error>(())
//! ```

// この crate は出力先へ書く口そのもの。ワークスペースの clippy.toml は std::fs の書き込み・削除・作成を
// 禁じている（setoka の 10-12 レビューの R8）ので、この crate だけ許す
#![allow(clippy::disallowed_methods)]
#![forbid(unsafe_code)]

mod guard;
mod orphans;
mod write;

pub use guard::{ensure_no_symlink_under, ensure_symlink_free, resolve_output_rel};
pub use orphans::{OrphanWarning, is_same_file, list_files_under, remove_orphans};
pub use write::{
    WriteOutcome, create_dir_all_under, remove_dir_all_under, remove_file_under,
    write_atomic_under, write_if_changed, write_under,
};
