---
name: publish-crate
description: 汎用ライブラリ（tankan / mikan / kabosu / shikuwasa / amanatsu / hebesu / jabara / kunenbo）を crates.io へ公開する手順。setoka のリリースとは非同期で、変更が溜まったときだけ行う。kabosu は publish 前の fuzz 必須。バージョンを上げて publish するときに使う。
---

# 汎用ライブラリの crates.io 公開

対象は monorepo 内の 8 crate。バージョンは workspace と独立で、各 `Cargo.toml` の
`version` を明示指定している（公開済み: tankan 0.2.2 / mikan 0.3.0 / kabosu 0.2.1 / shikuwasa 0.1.0 / amanatsu 0.1.0 / hebesu 0.1.0 / jabara 0.1.0 / kunenbo 0.1.0）。

| crate | 役割 | 備考 |
| --- | --- | --- |
| tankan | Mermaid 互換 SSR | |
| mikan | 検索エンジン | native / wasm で 1 実装共有。版を上げると検索 wasm も作り直す（CI の `search wasm inputs`）。**publish の前に `crates/mikan/clippy.toml` を消し、`lib.rs` に `#![cfg_attr(test, allow(clippy::disallowed_methods))]` を足す**（v0.22 Phase 106 で当面ワークスペースの書き込みの口の縛りを外した。src を変えるので wasm の作り直しと同じ PR で行う。ROADMAP「その他の候補」） |
| kabosu | TOML パーサ（依存ゼロ・no_std + alloc） | **publish 前に fuzz を必ず一度回す** |
| jabara | 静的サイトの開発サーバ（配信・ライブリロード・監視・Host / Origin の検査。v0.22 Phase 105 で setoka-server を改名） | serve のテストは TCP をバインドする（サンドボックスの外で動かす）。文言は日本語で setoka のコマンドや設定に触れない（setoka の案内は setoka-cli が足す）。`ServeOptions` は `#[non_exhaustive]` なので、項目を足しても破壊的変更にならない |
| hebesu | 日本語の表記ゆれの検査（依存ゼロ。v0.22 Phase 104） | ルールの名前は setoka の診断の名前と同じ（setoka-core の lint.rs のテストが縛る）。文言は日本語。単独の CLI は v0.23 の候補 |
| amanatsu | OpenAPI / JSON Schema の静的 HTML 化（v0.22 Phase 103） | crates.io の shikuwasa（json）に依存する（ワークスペースの `shikuwasa` の依存に `version` を書いてある）。shikuwasa の版を上げて新しい API を使ったら、**先に shikuwasa を publish** してから amanatsu を publish する。出力の文言は日本語・CSS は同梱しない（クラスの一覧は README） |
| kunenbo | 出力先への安全な書き込み（相対パスの検証・経路のシンボリックリンクの検査・残ったファイルの掃除。v0.22 Phase 106 で setoka-core の output.rs から切り出した） | ログは出さない（掃除の警告はコールバックで渡す。setoka-core の `output::remove_orphans` が切り出しの前と同じ文言でログに出す）。ワークスペースの `clippy.toml` が std::fs の書き込みを禁じているので、crate のルートで `clippy::disallowed_methods` を許している |
| shikuwasa | YAML の安全な読み込み（saphyr の上。v0.22 Phase 102） | 0.1.0 は publish 前に fuzz を回した（11-23）。feature `json` の `to_json` は serde_json を引く。saphyr を上げたら別名の上限（loader の `alias_limit`）と文言の対応表（`load.rs` の `scan_message`）を見直す |

公開しない crate: **mikan-wasm**（wasm 成果物を作るビルド用。`publish = false`）と
setoka 本体側の crate（将来構想は ROADMAP.md）。

## 手順

```bash
# 1. 対象 crate の Cargo.toml の version を上げ、Cargo.lock を追随させる
cargo build
# 2. コミット → push → CI green を確認（ci.yml の cargo package --locked が
#    メタデータ・同梱内容の回帰を検出する。作業ツリーが dirty だと package は拒否される）
# 3. kabosu は必須・shikuwasa・mikan・tankan も推奨: fuzz を一度回す（fuzz.yml を手動実行するのが基本。
#    週 1 回の定期実行の結果が新しければそれでもよい）
gh workflow run fuzz.yml          # 1 対象 600 秒。結果は Actions の Summary（panic・ハング・メモリ超過の件数）
#    手元で回すときは fuzz.yml の env（FUZZ_TIMEOUT・FUZZ_RSS_LIMIT_MB）と同じ値を渡す
#    （nightly・cargo-fuzz・C++ コンパイラが要る。開発コンテナには g++ が無い）
cd crates/kabosu && cargo +nightly fuzz run parse -- -max_total_time=600 -timeout=10 -rss_limit_mb=2048   # roundtrip / decode も（shikuwasa は crates/shikuwasa の parse）
# 4. dry-run → publish（要 cargo login。公開は取り消し不可・yank のみ可能）
#    mikan は crate の clippy.toml を消してあること（v0.22 Phase 106 の当面の措置。何も出なければよい）
cargo package --locked --list -p mikan | grep -x clippy.toml
cargo publish --dry-run -p <crate>
cargo publish -p <crate>
```

## 罠

- **公開は取り消せない**（yank で新規取得を止められるだけ）。dry-run と CI green を飛ばさない
- kabosu と hebesu は `[dependencies]` セクションを意図的に書かない = 依存ゼロ。CI が package 後の
  manifest を検査する（依存を足すと落ちる）
- setoka 側の `Cargo.lock` はバンプの `cargo build` で追随する。setoka のリリースタグとは
  無関係に進めてよい（非同期）
- 8 crate とも crate のディレクトリに `LICENSE-MIT` / `LICENSE-APACHE` の**コピー**を置いている
  （`license.workspace = true` だけでは package にライセンス文が入らない。シンボリックリンクは
  Windows の checkout で壊れるので使わない）。ワークスペース直下の LICENSE を変えたら 8 つとも
  合わせる。mikan は分かち書きモデルのライセンス文（`assets/model/LICENSE-*`）も同梱する
- tankan のテスト用コーパス（`tests/corpus`・`corpus_test.rs`・`snapshots`）は `exclude` で
  配布物から外している（kabosu の toml-test と同じ形。CI が `cargo package --list` で確かめる）
- 公開後は CLAUDE.md / README / docs（`development/kabosu.md` 等）の版表記を追随させる
