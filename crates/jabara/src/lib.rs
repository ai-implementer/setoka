//! # jabara
//!
//! 静的サイトの**開発サーバ**。ビルドした静的サイトのディレクトリを配信し、ファイルの監視と WebSocket の
//! ライブリロードで、編集 → 再ビルド → ブラウザの再読み込みをつなぐ。再ビルドそのものは使う側が
//! コールバックで渡す（この crate はビルドの中身を知らない）。v0.22 Phase 105 で setoka-server を改名して
//! 切り出した。
//!
//! - [`serve`] — ディレクトリを配信する（axum + tower-http の `ServeDir`。ブロッキング）
//!   - `guide/` → `guide/index.html` の解決・`base_url` の下での配信（`/` からリダイレクト）・
//!     無いページは `404.html` を 404 で返す（GitHub Pages と同じ）
//!   - [`ReloadNotifier`] を渡すと WebSocket のエンドポイント（既定は [`LIVERELOAD_PATH`]）を生やし、
//!     [`ReloadNotifier::notify`] で接続中のページへ `reload` を送る
//!   - Host / Origin の検査（DNS リバインディング対策。`localhost`・`*.localhost`・IP アドレスと
//!     `allowed_hosts` だけを通す）と、[`PathGuard`] によるシンボリックリンクの遮断
//! - [`watch`] — ディレクトリの監視（notify）。内容を変えない種類のイベント（開いた・読んだだけ）は捨て、
//!   静かになるまで集めてからコールバックを呼ぶ。[`WatchIgnore`] で出力ディレクトリ・隠しディレクトリ・
//!   任意の述語を除外する
//!
//! 文言（エラー・ログ）は日本語。

mod error;
mod host;
mod livereload;
mod serve;
mod watch;

pub use error::ServerError;
pub use livereload::{LIVERELOAD_PATH, ReloadNotifier};
pub use serve::{PathGuard, ServeOptions, base_path, serve};
pub use watch::{WatchFailure, WatchHandle, WatchIgnore, watch};
