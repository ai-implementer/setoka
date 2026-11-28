# ロードマップ

setoka の開発計画（今の版の Phase・候補）と、これまでの版の 1 行の要約。**このファイルが Phase 状態の正**
（README には現在の版と概要だけを置く）。リリース済みの版の内訳と、レビュー・見直しの記録（中身と
判断の経緯）は [ROADMAP-history.md](ROADMAP-history.md)。レビュー・見直しの未着手の項目は、下の
「レビューからの持ち越し（未着手）」で状態を管理する。

## 現在: v0.23（Phase 108〜115）

**v0.22.0 まで公開済み**。kabosu 0.2.1 / tankan 0.2.2 / mikan 0.3.0 と、v0.22 で切り出した shikuwasa / amanatsu / hebesu / jabara /
kunenbo（どれも 0.1.0）は crates.io で公開済み（setoka のリリースとは非同期。手順は `publish-crate` スキル）。v0.22 の計画と
各 Phase の記録は ROADMAP-history.md の「完了済み: v0.22（Phase 100〜107）の内訳」。

次は **v0.23**（テーマ「**依存の追随とコードの整理**」）を 8 つの Phase で進める。11-28 に策定した。v0.22 の策定（11-17）で
依存の追随とコードの整理を v0.23 の予定にしており、11-28 に依存とコードの今の状態を調べて Phase を決めた（下の「現状の確認」）。
テーマと範囲は仮決め（下の「v0.23 の方針」）。

### v0.23 の方針

範囲の外に出た依存の新しい版（8 件。うち 2 つは同梱の JS）に追随し、10-12 レビュー・10-07 見直し・10-04 見直しから持ち越した
コードの整理を片付ける。出力を変えない整理を先に済ませ、本文の組み立てを 1 か所にまとめてから、出力が変わりうる依存の更新を
1 つずつ行う。

- **仮決め**（11-28 に判断点を聞いたが応答が無かったので、推奨案で進める。計画の PR で確かめる）
  - テーマは「依存の追随とコードの整理」の両方にする。性能（10-07 見直しの E5）と新しい機能は v0.24 の候補にする
  - ほかの項目は足さない。v0.22 で v0.23 の予定にした次の 3 つは v0.24 の候補に移す: ci.yml のシェルの e2e を
    Windows でも動かす（e2e のスクリプト化を含む。v0.22.0 のリリースノートには「v0.23 へ延期した」と書いた）・hebesu を
    単独のコマンドとして出す・client 描画の Mermaid の書き間違いを check で報告する

以下は策定の詳細。

- 動機
  - 依存の新しい版が範囲の外に 8 件ある（下の「現状の確認」）。dependabot は minor / patch だけを組にまとめる設定なので、
    12 月の初めの月次の実行で comrak・minijinja・saphyr・notify の PR が 1 本ずつ来る見込み。vaporetto と wasm-bindgen は
    dependabot の ignore に入れてあり、手で上げないと古いまま残る
  - mermaid 11 系の最新（11.17.2）に入っている DOMPurify・js-yaml・KaTeX の勧告 8 件を、影響しない理由を書いて
    `scripts/vendor-advisories.sh` で除外している。12 系に上げると除外を外せる見込み
  - 10-12 レビューの R6・R7・R10 の残り、10-07 見直しの E4、10-04 見直しの「同じ処理が 2 か所にある」を、v0.22 で
    v0.23 の予定にした。R6（本文の組み立て）は comrak の更新で直す所と同じなので、先に整理しておくと comrak の
    差分を読みやすい
- ゴール: 範囲の外の依存 8 件が新しい版になり（延期したものはリリースノートに書く）、`cargo update --dry-run` で更新が
  無い。持ち越しの整理 5 件（R6・R7・R10 の残り・E4・同じ処理が 2 か所）が片付いている。v0.22.0 からの出力の差分は
  依存の更新から来る意図したものだけで、理由を記録している
- 変わるもの
  - setoka-core の本文の組み立て（`markdown/`）と合成ページの型（`Page`）。setoka の crate は公開しないので、
    crate の間の API は変えてよい
  - 依存の版と Cargo.lock・同梱の JS（mermaid・KaTeX）・検索のモデルと wasm・第三者のライセンスの通知・
    `dependabot.yml` の ignore・vendor の勧告の除外
  - 公開している crate の版: shikuwasa・jabara・mikan と、shikuwasa の版しだいで amanatsu（tankan は 110 の判断しだい）。
    jabara は公開のエラー `ServerError::Notify` が `notify::Error` を持つので 0.2.0。shikuwasa は saphyr の型を公開 API に
    出していない（11-28 に確認）ので、動作が変わらなければ 0.1.1 で足りる（変わるなら 0.2.0）。mikan は利用者が渡す
    モデルの形式が変わるので 0.4.0
  - 出力: comrak・mermaid・KaTeX・分かち書きの変化の分だけ変わりうる（minijinja は変えない見込み）。本文 HTML か
    検索 tf が変わったら `CACHE_FORMAT_VERSION` を上げる。MSRV（1.99）は据え置く見込み（新しい版がより新しい rustc を
    要るなら判断点にする）
- Phase は「108 本文の組み立ての整理 → 109 合成ページの型 → 110 小さい整理 → 111 comrak 1.x → 112 minijinja 3 →
  113 汎用ライブラリの依存 → 114 検索 → 115 図と数式」の順
  - 108〜110 は出力を変えない整理。先に済ませると、111 以降の差分が依存の更新から来たものだけになる
  - 108 を 111 より前に置くのは、comrak の API の変更で直す所（ノードの生成と組み替え）を 108 で 1 か所にまとめるため
  - 113 は publish の順序がある（amanatsu は crates.io の shikuwasa に依存するので、shikuwasa の版を 0.2.0 に上げるなら、
    shikuwasa を publish してから amanatsu の依存を上げる。PR を 2 本に分ける）
  - 114 は検索 wasm の作り直しを 1 回にまとめる（vaporetto 0.7・wasm-bindgen 0.2.130・mikan の `clippy.toml` の解消・
    `make_excerpt` のテストは、どれも wasm の作り直しが要る）
  - 115 を最後に置くのは、見た目の確認をユーザがブラウザで行うため（ほかの Phase をその確認で待たせない）
  - dependabot の major の PR（12 月の初めに来る見込み）は、対応する Phase で片付けて閉じる。Phase より前に来た PR は
    マージせずに置いておく
  - 着手時に判断点を決めてから実装する。応答が無いまま推奨案で進めた判断点は「仮決め」と PR と ROADMAP に書く
- 入れないもの（v0.24 の候補）: 上の仮決めで移した 3 つ（Windows のシェルの e2e とスクリプト化・hebesu の CLI・
  client 描画の Mermaid の check）・10-07 見直しの E5（静的ファイルを毎回読む・sha256 を何度も計算する）・API 仕様の
  check で文書の中の参照も見る・書き手の機能（非破壊 `init`・PR 検証 CI の雛形・Admonition のラベル）・長い関数
  （R6 で `render_body_html` は短くなる）・性能の気になる点・E1・F2・F3・F5・F8
- **リリース判定**（すべて満たしたらリリースする。どれもタグの前に確かめられる）
  - 範囲の外の依存 8 件が新しい版になっている（延期したものはリリースノートに書き、判定から外す）。
    `cargo update --dry-run` で更新が無く、dependabot の開いた PR が無く、cargo-deny と deps.yml が通る（111〜115）
  - 「レビューからの持ち越し」の R6・R7・E4・R10・同じ処理が 2 か所 が、取り消し線と行き先になっている（108〜110・114）
  - v0.22.0 の setoka と今の setoka で docs と雛形のサイトの dist を比べ、差分がすべて意図した変化で、理由を
    記録している（108〜110 と 113 は差分が無い）
  - ブラウザのテスト（`tests/browser/`）・test-cross（Windows・macOS）・docs ゲート・`docs_excerpt` が通る
  - 仮決めのままの判断点が無い（計画の 2 つの仮決めを含む。残すならリリースノートに書く）
  - crates.io への公開は判定に入れない（setoka のリリースと非同期。setoka はワークスペースの path で使う）
- **非互換と注意に書く見込みのもの**
  - comrak・mermaid・KaTeX の更新による、本文の HTML と図・数式の見た目の変化。comrak の整形の結果が変われば、
    `setoka fmt` が原稿を書き換える（`setoka check` が整形の差分を報告する）
  - minijinja 3 で書き方が変わった構文やフィルタを、`theme/` で上書きしたテンプレートが使っていれば動かなくなる
  - mermaid 12 の必要なブラウザ（Safari 17.4 以降）と、`mermaid.min.js` の大きさ（通常版なら gzip 後で約 500 kB 増える）
  - 検索のモデルの形式が変わるので、読者のブラウザに古い検索 wasm が残っていると、HTTP のキャッシュが切れるまで
    検索が止まる
  - `CACHE_FORMAT_VERSION` を上げたら、最初のビルドが全ページを作り直す
- **ユーザの作業**

  | 作業 | Phase | 時期 |
  | --- | --- | --- |
  | テーマと範囲（2 つの仮決め）を計画の PR で確かめる | — | 計画の PR のレビュー |
  | 各 Phase の判断点を決める | 108〜115 | 着手時 |
  | shikuwasa・jabara を publish する（`publish-crate` スキル）。shikuwasa を 0.2.0 にしたら、その後に amanatsu も | 113 | 113 の PR（2 本なら 1 本目と 2 本目）のマージ後 |
  | mikan を publish する（tankan は 110 で版を上げたら） | 110・114 | それぞれのマージ後（まとめてもよい） |
  | mermaid・KaTeX の見た目をブラウザで確かめる | 115 | 115 の PR |

### 現状の確認（11-28）

- 依存（main 703d675）: semver の範囲の中は Cargo.lock に入っている（`cargo update --dry-run` で 0 件）。勧告・issue・
  開いた PR は 0 件。範囲の外の新しい版は次の 8 件

  | 依存 | 今 | 新しい版 | 使う crate | 注意 |
  | --- | --- | --- | --- | --- |
  | comrak | 0.59 | 1.2.0 | setoka-core | 凍結した選択（版は上げてよい）。本文 HTML が変わりうる |
  | minijinja | 2.21 | 3.1.0 | setoka-render | テーマのテンプレートを上書きしている利用者に関わる |
  | saphyr / saphyr-parser | 0.3.3 | 0.4.1 | shikuwasa（→ amanatsu） | frontmatter と API 仕様の読み方 |
  | notify | 8.2 | 9.0.0 | jabara | 8.2 に留めていたのは 9 が rc だったため |
  | vaporetto / vaporetto_rules | 0.6 | 0.7.0 | mikan | モデルの形式が変わる。dependabot で ignore 中 |
  | wasm-bindgen | =0.2.126 | 0.2.130 | mikan-wasm | 版を完全一致で固定。dependabot で ignore 中 |
  | mermaid（同梱の JS） | 11.17.2 | 12.4.0 | setoka-theme | 下の「今後の候補」の「mermaid 12 系への更新」 |
  | KaTeX（同梱の JS） | 0.18.11 | 0.19.1 | setoka-theme | 数式の見た目 |

- コードの整理（main 703d675 で数えた）
  - R6: `render_body_html`（`crates/setoka-core/src/markdown/mod.rs`）は 292 行。`NodeHtmlBlock` の生成が 5 か所・
    `arena.alloc(AstNode::new(..))` が 4 か所あり、同じ 11 行のクロージャを 2 回定義している。comrak の API を使うのは
    `markdown/` の 6 ファイル（`mod.rs`・`collapse.rs`・`fragment.rs`・`heading.rs`・`reading.rs`・`tabs.rs`）で、
    CLAUDE.md の「`markdown/mod.rs` が comrak を触る唯一の場所」は今の実装と合わない
  - R7: 否定のフィルタ（`!….is_generated()`）が、テストを除くコードに 24 か所（setoka-core 11・setoka-cli 10・
    setoka-render 3。10-12 は 21 か所）
  - E4: `rel_to_slash`（`urlpath.rs`）を使わずに rel を `/` 区切りにしている所が 8 か所（`cache.rs` の `rel_str` は
    中身が同じ写し）
  - R10 の残り: `build_once` の同義語の合成と丸めにテストが無い / `resolve.rs` の保護対象の配列とエラー文言の match が
    別で、`_ =>` が `.setoka` の文言 / `BuildCache::meta(&str, &str)` / `collect_sections` の `sections.last_mut().unwrap()`
    が 6 か所 / tankan の前置き行を飛ばす処理が 11 か所・frontmatter の `title:` の扱いが 10 か所・SVG のルート要素が
    9 か所 / mikan の `make_excerpt`（135 行）に参照実装と比べるテストが無い。cli の run 関数のテストは v0.22 Phase 101 で
    済んだ
  - 同じ処理が 2 か所: `escape_html` が amanatsu（`lib.rs`）と setoka-core（`markdown/mod.rs`）にある。mermaid / math の
    言語の集合は、setoka-core の `is_special_render_lang` と setoka-render の `highlight.rs` の分岐をコメントで同期させて
    いるだけ（openapi / jsonschema は `SPEC_LANGS` とテストで縛っている）
  - E5（v0.24 の候補）: 静的ファイルを、変わっていなくても毎ビルド全部読む（1 ファイルで最大 2 回）。原稿の sha256 を
    1 ページで 4〜7 回計算する

### 108 本文の組み立ての整理 ⬜

**概要**: `render_body_html` のノードの生成を 1 つの関数にまとめ、記法ごとの組み替えを各記法のモジュールへ移す
（10-12 レビューの R6）。あわせて 10-04 見直しの「同じ処理が 2 か所にある」の残り（`escape_html` と、mermaid / math の
言語の集合）を片付ける。出力は変えない。

- 現状: 上の「現状の確認」の R6 と「同じ処理が 2 か所」。タブ・折りたたみ・用語集の解析は `tabs.rs`・`collapse.rs`・
  `glossary.rs` にあるのに、組み替えは `mod.rs` にあり、記法を 1 つ変えると 2 ファイルを直す
- やること
  - ノードの生成（`HtmlBlock` への差し替えと `arena.alloc`）を 1 つの関数にまとめ、2 回ある同じクロージャを 1 つにする
  - 組み替えを各記法のモジュールへ移す。**適用の順序は `render_body_html` に残す**（タブの判定は展開の前・分量は
    展開の後に数える・用語集の `<abbr>` はほかの適用がすべて終わった後、の制約が記法をまたぐため）。本体は約 150 行に
    なる見込み
  - `escape_html` の 2 つの扱いを決める。setoka-core は amanatsu に依存しない（依存の向きは凍結）ので、1 つにするには
    向きを変えることになる
  - mermaid / math の言語を enum にし、setoka-render の分岐を網羅の match にする
  - CLAUDE.md の「`markdown/mod.rs` が comrak を触る唯一の場所」を「`markdown/` の配下だけ」に直す
  - 出力が変わらないことを、スナップショット・キャッシュの指紋・docs と雛形のサイトの dist で確かめる
- 判断点: ノードの生成の関数の引数と、各記法のモジュールから呼ぶ口 / `escape_html` を 2 つのままにして、両方のテストに
  同じ入力と期待値の表を置くか、setoka-core が amanatsu に依存するか（CLAUDE.md の凍結した依存の向きを変えることになる）
- 非互換と注意: なし（出力を変えない）
- 工数: 中

### 109 合成ページの型 ⬜

**概要**: 合成ページ（用語集・検索結果ページ）の `Page.src` を、実在するファイルのパスとして取り出せなくする（10-12
レビューの R7）。書き込みや診断の経路で除外を書き忘れたら、コンパイルが止まるようにする。出力は変えない。

- 現状: 合成ページの `Page.src` は実在しないパスで、`fmt` / `lint --fix` の書き込み・lint・診断のリンク元などから
  外すのは、否定のフィルタ（`!….is_generated()`）の 24 か所（上の「現状の確認」）。書き忘れてもコンパイルが通る
  （CLAUDE.md の「合成ページ」）
- やること: ページの出どころを `PageOrigin { File(PathBuf), Generated(GeneratedKind) }` にし、ファイルのパスは
  `source_file() -> Option<&Path>` からだけ取れるようにする。否定のフィルタ 24 か所を、`source_file()` が `Some` の
  ページだけを扱う書き方に直す。CLAUDE.md の「合成ページ」の節を直す
- 判断点: `Page` のフィールド（`src`・`generated`）の変え方（setoka-render・setoka-index・setoka-cli が使う）/
  リンク元の表示などパスの文字列だけが要る所（`rel`）をどうするか
- 非互換と注意: なし（出力を変えない）
- 工数: 中

### 110 小さい整理 ⬜

**概要**: 10-07 見直しの E4（rel を `/` 区切りにする変換を `rel_to_slash` の 1 つにする）と、10-12 レビューの R10 の
残り（mikan の `make_excerpt` は 114 で行う）を片付ける。出力は変えない。

- 現状: 上の「現状の確認」の E4 と R10 の残り
- やること
  - E4: 手書きの変換 8 か所と `cache.rs` の `rel_str` を `rel_to_slash` に替える
  - `build_once` の同義語の合成と丸め（`IndexParams` の組み立て）を純関数に出してテストを書く
  - `setoka_config::load` の保護対象の配列にエラー文言を入れ、`_ =>` の文言を無くす
  - `BuildCache::meta` の 2 つの `&str` を型（`SourceHash` など）で分ける
  - `collect_sections` の `sections.last_mut().unwrap()` 6 か所を、状態の構造体にして消す
  - tankan: 前置き行を飛ばす処理（11 か所）・frontmatter の `title:` の扱い（10 か所）・SVG のルート要素（9 か所）を
    `common/` の 1 つずつにまとめる。今の動作のまま（`title:` の引用符を外すのは packet だけ）まとめ、SVG が変わらない
    ことを corpus のスナップショットで確かめる
- 判断点: tankan の共通化の範囲と版（動作を変えなければ 0.2.3。`title:` の引用符の扱いを図種の間でそろえると、
  引用符付きの title の SSR の図が変わるので、そろえるなら 0.3.0 にして setoka は `CACHE_FORMAT_VERSION` を上げる）/
  drafts・force の bool が 7 段そのまま渡る件も扱うか
- 非互換と注意: なし（setoka の出力を変えない）
- 工数: 中（それぞれは小）

### 111 comrak 1.x ⬜

**概要**: comrak を 0.59 から 1.x（11-28 の最新は 1.2.0）へ上げる。本文 HTML が変わりうるので、スナップショットの
差分を目で確かめ、変わったら `CACHE_FORMAT_VERSION` を上げる。

- 現状: comrak は凍結した選択（CLAUDE.md の「凍結した設計判断」）で、onig を引かないよう `default-features = false`
  にしている。dependabot の組 `render-output` は minor / patch だけなので、1.x の PR は別に来る見込み。これまでの更新
  （0.55 → 0.56・0.56 → 0.59）では autolink の解釈・位置情報・テーブルの描画が変わり、`CACHE_FORMAT_VERSION` を
  上げた（`cache.rs` の記録）
- やること: 0.59 から 1.2 までの変更点（上流の CHANGELOG）を読み、API の変更（オプションの構造体・ノードの型・
  `format_html` 系）に追随する。`default-features = false` で onig を引かないこと（`deny.toml` の bans）と、使っている
  拡張（`cjk_friendly_emphasis`・front matter・脚注など）が残っていることを確かめる。整形（`format_commonmark`）の
  結果が変わると利用者の原稿が `setoka fmt --check` で差分になるので、docs と雛形の原稿で確かめる。CLAUDE.md の
  comrak の注意（`render.r#unsafe`・`header_id_prefix` など）を直す
- 判断点: 変わる出力（本文 HTML と整形の結果）を受け入れるか / 1.x で増えた拡張を使うか（使うなら別の Phase にする）
- 非互換と注意: 本文 HTML の変化 / 整形の結果が変われば `setoka fmt` が原稿を書き換える / `CACHE_FORMAT_VERSION` を
  上げれば最初のビルドが全ページを作り直す
- 工数: 中

### 112 minijinja 3 ⬜

**概要**: minijinja を 2.21 から 3.x（11-28 の最新は 3.1.0）へ上げる。テーマのテンプレートと自前のフィルタ（`| url`・
`| url_js` など）を追随させ、`theme/` でテンプレートを上書きしている利用者に直す所を案内する。

- 現状: minijinja は凍結した選択。テンプレートは setoka-theme の `templates/`（`base.jinja` など）にあり、利用者は
  プロジェクトの `theme/` に同じ相対パスで置くとファイル単位で上書きできる。上書きしたテンプレートも同じ minijinja が
  読むので、構文やフィルタの動作が変わると、上書きしている利用者のサイトのビルドが失敗するか HTML が変わる。互換性の
  ページはデフォルトテーマの中身を約束の対象外にしているが、テンプレートの言語（minijinja）が変わることは書いていない
- やること: 2.21 から 3.1 までの変更点を読み、`setoka-render/src/templates.rs`（Environment・ローダ・自前のフィルタ）と
  テーマのテンプレートを追随させる。HTML が変わらないことをスナップショットで確かめる（自動エスケープの既定や `/` の
  エスケープが変わっていないか）。互換性のページ（`docs/content/reference/compatibility.md`）と「見た目の調整」に、
  テンプレートの言語の版が変わりうることと、上書きした利用者が直す所を書く
- 判断点: 非互換の伝え方（リリースノートだけか、docs にも書くか）/ HTML が変わったときに受け入れるか
- 非互換と注意: `theme/` で上書きしたテンプレートが、3 で変わった構文やフィルタを使っていれば動かなくなる（テンプレートの
  出力はキャッシュしないので、`CACHE_FORMAT_VERSION` は上げない）
- 工数: 小〜中

### 113 汎用ライブラリの依存 ⬜

**概要**: 公開している crate の依存を上げる。shikuwasa の saphyr / saphyr-parser を 0.4 に、jabara の notify を 9 にする
（shikuwasa の版しだいで amanatsu も）。setoka の出力は変えない見込み。

- 現状: shikuwasa は saphyr の上に、別名の展開の上限・同名キーの検出・位置のバイトへの換算・スカラーの解釈を足している
  （CLAUDE.md の「YAML の読み込み」）。saphyr の位置の数え方・`early_parse`・別名の扱いが 0.4 で変わると、これらの前提が
  崩れる。jabara は notify で監視し、イベントの種類で絞っている（Linux の inotify は開いた・読んだだけのイベントも
  届ける）。shikuwasa は saphyr の型を公開 API に出していない。jabara は公開のエラー `ServerError::Notify` が
  `notify::Error` を持つ（11-28 に確認）
- やること
  - 1 本目の PR: shikuwasa の saphyr を 0.4 にし、shikuwasa のテスト・fuzz（fuzz.yml の shikuwasa の `parse` を手で 1 回）・
    setoka のスナップショットで読み方が変わらないことを確かめる。jabara の notify を 9 にし、setoka-cli の結合テスト
    （`build --watch`・`dev`。test-cross で Windows・macOS でも動く）で監視が動くことを確かめる。shikuwasa と jabara の
    版を上げる
  - shikuwasa を 0.2.0 にしたら: shikuwasa の publish（ユーザ）の後に 2 本目の PR で amanatsu の依存を上げ、amanatsu の
    版を上げる（0.1.1 なら、ワークスペースの `version = "0.1.0"` の指定のままで入るので、2 本目は要らない）
- 判断点: shikuwasa の版（動作が変わらなければ 0.1.1、変わるなら 0.2.0。jabara は公開のエラーの型が変わるので 0.2.0）/
  saphyr 0.4 で読み方が変わったときに、shikuwasa で吸収するか、変わった読み方を受け入れるか（frontmatter のキーと値の
  読み方は互換性の約束の対象）
- 非互換と注意: なし（読み方を変えない見込み。変わるならマイナー版の今回で書く）
- 工数: 中

### 114 検索 ⬜

**概要**: mikan の vaporetto を 0.7 にし（モデルを 0.7 の形式に差し替える）、mikan-wasm の wasm-bindgen を 0.2.130 にして、
検索 wasm を 1 回で作り直す。同じ作り直しで、mikan の `clippy.toml` の解消と `make_excerpt` のテスト（R10 の残り）も
行う。mikan は 0.4.0 にする。

- 現状: 下の「今後の候補」の「vaporetto 0.7 系への更新」と「mikan も書き込みの口の縛りに入れる」。vaporetto と
  wasm-bindgen は dependabot の ignore にある。wasm-bindgen は wasm-bindgen-cli と完全に同じ版が要る
  （`scripts/build-search-wasm.sh`）。mikan の src を変えると、wasm の入力のハッシュ（`wasm-inputs.sha256`）が変わる
- やること
  - `scripts/vendor-vaporetto-model.sh` の取得元を vaporetto 本体の v0.7.0 のリリースに添付されたモデルに替え、SHA-256 を
    差し替える。vaporetto と vaporetto_rules を同じ版にし、`dependabot.yml` の vaporetto の ignore を外す
  - wasm-bindgen を 0.2.130 にし、wasm-bindgen-cli も同じ版にする（`vendor-update` スキル）。wasm-bindgen の ignore は
    残す（版の完全一致が要るのは変わらない）
  - mikan の `lib.rs` に `#![cfg_attr(test, allow(clippy::disallowed_methods))]` を足して `crates/mikan/clippy.toml` を消す
  - `make_excerpt`（135 行）の窓を選ぶ処理を出し、総当たりの参照実装と比べるテストを書く
  - 検索 wasm と第三者のライセンスの通知を作り直す（`scripts/third-party-licenses.sh`）
  - 分かち書きが変わらないことを `setoka search` とキャッシュの指紋で確かめる。変わったら `CACHE_FORMAT_VERSION` を上げる。
    索引の `FORMAT_VERSION` は上げない
- 判断点: モデルの取得元（vaporetto 本体のリリース / ほかの配布元）/ 分かち書きが変わったときの扱い（受け入れるか、
  0.6 の形式のモデルを変換して同じ分かち書きを保つか）/ 古い検索 wasm が残る読者への対策を入れるか
- 非互換と注意: 読者のブラウザに古い検索 wasm が残っていると、新しいモデルを読めず、HTTP のキャッシュが切れるまで
  検索が止まる / mikan 0.4.0 はモデルの形式が変わる（mikan に自分でモデルを渡している利用者は 0.7 の形式に替える）
- 工数: 中

### 115 図と数式 ⬜

**概要**: 同梱の mermaid を 12 系（11-28 の最新は 12.4.0）、KaTeX を 0.19 系（同じく 0.19.1）へ上げる。ブラウザの
テスト（`tests/browser/`）と vendor の勧告の除外を見直す。図と数式の見た目が変わるので、ユーザがブラウザで確かめる。

- 現状: 下の「今後の候補」の「mermaid 12 系への更新」（11-17 に確かめたこと: 既定のレイアウトが ELK・gzip 後で約 500 kB
  増える・Safari 17.4 以降・tiny 版の制限・同梱の elkjs のライセンス）。KaTeX は数式のクライアント描画に使う
- やること: `scripts/vendor-mermaid.sh` / `scripts/vendor-katex.sh` で差し替え、第三者のライセンスの通知を作り直す
  （通常版なら elkjs の「EPL-2.0 OR GPL-3.0-or-later」が入る）。`scripts/vendor-advisories.sh` の除外 8 件を外して照合
  し直す。`mermaid-init.js` の初期化（`layout`・`look`）と配色の追従を確かめる。ブラウザのテストの Mermaid（雛形の
  「記法の例」の 9 つ）と KaTeX（3 つ）を通す
- 判断点: 12 の通常版 / tiny 版 / 11 のまま / 見た目を tankan の SSR の図とそろえるか（`layout: dagre`・`look: classic`
  で前の見た目を保つか）/ KaTeX 0.19 で数式の見た目が変わったときに受け入れるか
- 非互換と注意: 図と数式の見た目の変化 / mermaid 12 の必要なブラウザ（Safari 17.4 以降）/ `mermaid.min.js` の大きさ /
  テーマの `mermaid-init.js` を上書きしている利用者
- 工数: 中

## 今後の候補

### dogfooding 候補（v0.13 Phase 61 からの持ち越し）

- ~~**OS ダーク追従** / **見出しパーマリンクのキーボード到達性** / **`<head>` メタ** /
  **ページメタの拡充（読了時間・文字数）**~~ — 4 件とも v0.18 の Phase 76・78・79 で
  実装済み（内訳は「これまでのリリース」の「完了済み: v0.18」）

### 10-07 見直しからの候補

- ~~**最初の外部利用者**（10-07 見直しの F1）~~ → v0.21 Phase 99（記録の手順と基準値）。露出は
  Phase 94〜98、観察は v0.21 のリリース後。外部の利用者を得て、その使い方を観察する
  - 現状（10-07 に確認）: Release の各アセットのダウンロード数は v0.16〜v0.18 で 1 回
    ずつで、これは release.yml の publish-release が SHA256SUMS を作るために
    `gh release download` した分。直近 14 日の閲覧は約 80 回・閲覧者 1 人、issue は
    累計 0 件（`gh api`）
  - 対処案: リリースごとに、自分の取得分を引いたダウンロード数と閲覧数をリリース行に
    記録する。FAQ と Issues への導線（v0.19 Phase 84）の後に、既知の「外部への露出」
    （英語の README・他ツールとの比較・cargo-binstall）とあわせて進める
- ~~**検索の順位**（10-07 見直しの D2）~~ → v0.20 Phase 89
- ~~**名前「yuzu」の調査**（10-07 見直しの F7）~~ → v0.21 Phase 94（11-15 にユーザが setoka へ
  変えると決めた）
  - 現状: GitHub でリポジトリ名に yuzu を含むものは約 2,850 件で、上位は Switch
    エミュレータの関連。crates.io の名前が取られている件は既知。商標は**未調査**
  - 対処案: 改名するか、区別のための副名を付けるかを早めに決める（URL・雛形・crate 名が
    増えるほど改名の費用が上がる）。商標を簡単に調べる
  - 調査は v0.20 の期間中にユーザが進める（v0.20 のユーザの作業。v0.21 の外部への露出の前提）

### PR #53 のレビューからの候補（10-14）

PR #53 のレビュアー側の協議で提案され、こちらの 3 人格の協議（v0.20 の方針を参照）でも
「需要はまだ仮説で、今ある機能と docs でおおむね足りる」と見た。どれも、最初の外部利用者の
導入を観察してから（上の「最初の外部利用者」）着手するかを判断する。v0.20 の Phase 91 には、
導入で実際に止まる所の修正と docs だけを入れた。再レビューは「非破壊 init → PR 検証 CI の生成 →
詳細な JSON」の順を提案したが、順序は観察で多かった止まり方から決める（3 人格とも、順序を先に
決めずに前提だけを書く意見）。前提は、init は `input.dir = "."` の扱い、JSON は互換性の方針
（v0.21 Phase 95）。v0.21 のリリース後に観察してから判断する。

- **既存 Markdown 向けの非破壊 `init`** — 原稿のディレクトリを指定し、最小の setoka.toml だけを
  作る（既存の設定があれば止まる。原稿の移動・変換・公開はしない）
  - 今あるもの: 手順は `guide/install.md` の「既存のフォルダで始める」にあり、空の setoka.toml に
    `input.dir` を書けば build できる（10-14 に確認）。自動にできるのは手順の最初の 2 つだけで、
    deploy.yml は渡らない。リポジトリ直下の原稿は `input.dir = "."` が設定エラーなので救えない
  - 戻せない決定: サブコマンドの名前と引数。直下の原稿を扱うなら、先に `input.dir = "."` の
    扱い（dist・.setoka・public・theme・README 類を走査から外す設計）を決める必要がある
- **公開物の詳細な JSON**（出力ファイルと、どの原稿・設定から出たか）— 人向けの 1 行の要約は
  v0.20 Phase 91 に入れた。JSON は `check --format json` と並ぶ戻せない契約になり、出力ごとの
  由来は出力マニフェスト（`setoka-core/src/output.rs`）に無いので新しく記録する必要がある。
  互換性の方針の後（v0.21 Phase 95 で `--format json` のキーは消さない約束にしたので、足す JSON もこの約束の対象になる）
- **PR 検証 CI の雛形の生成** — docs の例は v0.20 Phase 91 に入れた。`setoka new` で生成するのは、
  既存の workflow を上書きしない作りと明示の選択が要るので、観察の後

### その他の候補

- **hebesu を単独のコマンドとして出す**（v0.24 の候補。v0.22 Phase 104 で延期を決め、v0.23 の計画で v0.24 へ移した = 仮決め）— Markdown（や素のテキスト）を
  読んで表記ゆれを報告・自動修正する CLI。Markdown を読むなら comrak に依存し、コードブロック・URL を除く決めごと
  （setoka-core の `extract_text_spans` と同じにするか）と、用語辞書の書き方（設定ファイル）を決める
- **API 仕様の検査（`setoka check` / amanatsu の `check`）で文書の中の参照も見る**（v0.22 Phase 103 のレビューで
  分かった）— 今は別ファイルへの `$ref` の読み込みとパースまでで、`#/components/schemas/Nope` のように指す先が無い
  参照や循環は、描画で注記になるだけで check は何も言わない。足すと check の出力が増える（新しい診断）。参照ファイルの
  上限に読めなかったファイルも数えるか（今は読めたファイルだけ）も一緒に決める
- **mikan も書き込みの口の縛りに入れる**（次に検索 wasm を作り直すとき = v0.23 Phase 114。v0.22 Phase 106 で決めた）— 今は
  `crates/mikan/clippy.toml` でワークスペースの `clippy.toml` の縛り（R8）を外している。vaporetto 0.7 への更新か mikan の版上げで
  wasm を作り直すときに、mikan の `lib.rs` に `#![cfg_attr(test, allow(clippy::disallowed_methods))]` を足して
  `crates/mikan/clippy.toml` を消す（src を変えるので wasm の作り直しと同じ PR にする）。mikan の publish より前に行う
- **ci.yml のシェルの e2e を Windows でも動かす**（v0.24 の候補。v0.22 Phase 101 で延期を決め、v0.23 の計画で v0.24 へ移した = 仮決め）— 今の
  Windows・macOS の CLI の確認は setoka-cli の結合テスト（`crates/setoka-cli/tests/`）だけで、ci.yml の e2e（bash。
  Linux だけ）が見ている build の報告・リンクの遮断・外部リンク検査などは Windows で動かしていない。パスと改行の
  違いで直す所が多い
  - あわせて、e2e を docs ゲート（`scripts/docs-gates.sh`）と同じくスクリプトにして、ci.yml と verify スキルから呼ぶか
    決める（v0.22 Phase 107 で延期した。verify スキルの 6 節は今も ci.yml の e2e の一部を手で写している）
- **client 描画の Mermaid の書き間違いを check で報告する**（v0.24 の候補。v0.23 の計画で v0.24 へ移した = 仮決め。setoka-core は tankan に依存できないので、検査は setoka-render 側に置き `rules::CHECK_ONLY` に足す。mermaid の版の判断（v0.23 Phase 115）の後）（v0.19 Phase 85 から移した）— 今は
  既定の client では build も check も何も言わない。tankan のパースだけ通し、未対応
  （`is_unsupported`）以外の構文エラーを抑制できる warning で報告する案。tankan が未対応の
  書き方を誤りとして報告しないよう、切り分け方を決めてから扱う
- **mermaid 12 系への更新**（v0.23 Phase 115） — v0.18.1 で 11 系に留めた（12.x は tankan の互換対象の版と
  あわせて決める）。Phase 82 の照合で、11 系の最新の 11.17.2 が束ねる DOMPurify・js-yaml・
  KaTeX に勧告が 5 件見つかり、影響しない理由を書いて `scripts/vendor-advisories.sh` で
  除外している（10-10）。11-06 に DOMPurify の 3 件（修正は 3.4.17）を同じく除外に足した（計 8 件）。
  12 系へ上げるときに除外を外して照合し直す
  - 11-17 に確かめたこと: 12.0.0（2026-09-10）は非互換の版で、レイアウトの既定が ELK になり
    （`mermaid.min.js` が gzip 後で約 500 kB 増える）、既定の見た目が変わる（`layout: dagre`・`look: classic` で
    前の見た目を保てる）。必要なブラウザは Safari 17.4 以降。最新の 12.3.0 の依存は DOMPurify ^3.4.17・
    KaTeX ^0.18.0 で、js-yaml は外れた（除外 8 件は外せる見込み）。同梱の elkjs 0.12.1 は「EPL-2.0 OR
    GPL-3.0-or-later」。ELK を含まない tiny 版（`@mermaid-js/tiny`）は mindmap・architecture・KaTeX・
    遅延読み込みに対応しない。判断点は「12 の通常版 / tiny 版 / 11 のまま」と、見た目を tankan の SSR の図と
    そろえるか
- **vaporetto 0.7 系への更新**（v0.23 Phase 114。vaporetto に依存しモデルを同梱しているのは公開している mikan なので、mikan の版も上がる） — 0.7.0（10-24）はモデルの形式が変わり、同梱のモデル
  （vaporetto-models v0.5.0 の `bccwj-suw_c1.0`。0.6 の形式）を `model version mismatch` で読めない。
  モデルの差し替えと検索 wasm の作り直しが要るので、自動では上げず手動で行う（11-06。dependabot の
  #74・#75 を閉じ、`dependabot.yml` の ignore に足した）
  - wasm を作り直すので、上の「mikan も書き込みの口の縛りに入れる」を同じ PR で行う
  - 0.7 の形式のモデルは vaporetto 本体の v0.7.0 のリリースに添付されている（`bccwj-suw_c1.0.tar.xz` も
    ある）。`scripts/vendor-vaporetto-model.sh` の取得元（今は vaporetto-models）と SHA-256 を差し替える
  - 上流のリリースノートでは、BCCWJ だけのモデルは 0.5.0 のものを `convert_bincode_model` で新しい
    形式へ変換したもの。分かち書きは変わらない見込みだが `setoka search` で確かめ、変わるなら
    `CACHE_FORMAT_VERSION` を上げる
  - vaporetto と vaporetto_rules は同じ版にする（片方だけだと 0.6 と 0.7 が両方入る）
  - 11-17 に確かめたこと: 0.7.0 は MIT OR Apache-2.0 で、MSRV を宣言していない。`bccwj-suw_c1.0` の 0.7 形式の
    モデルがリリースに添付されている。モデルの形式が変わるので、読者のブラウザに古い検索 wasm が残っていると
    新しいモデルを読めず、キャッシュが切れるまで検索が止まる（リリースノートの「非互換と注意」に書く）
- ~~**CLI の e2e を Windows でも実行する**~~ → v0.22 Phase 101（延期可）（v0.20 Phase 87 から移した）— Windows・macOS は
  `cargo test` だけで、ci.yml の e2e（`setoka new` → build → check）は ubuntu だけ
- **i18n** — テーマ UI 文字列の多言語化（v0.21 の対象の読者は当面日本語だけ。英語に変える可能性は
  ある = 11-15 にユーザが決めた。Phase 98）
  - Admonition のラベル（NOTE・TIP 等の見出しの文言）もここで扱う（v0.20 Phase 90 から移した）
  - 規模（実測）: jinja 18 ＋ テーマ JS 19 ＋ apispec 35 ＋ crossref 3 文字列。
    `site.lang` は `<html lang>` の 2 箇所でしか使われていない
  - 半端になる懸念: **検索（vaporetto の分かち書き）と `lint.rules`（全角英数・
    半角カナ・長音符）は日本語固有**なので、UI だけ多言語化しても中途半端
  - 代替にならない: テーマ上書き（`theme/templates/`）で文言は今でも変えられるが、
    粒度がファイル単位でアップストリームから fork することになる
    （`search-ui.js` は 470 行）
  - 最小案: `theme.strings` の部分上書き辞書（`theme.css_vars` / `glossary.terms` と
    同型）。既定を日本語のまま据え置けばスナップショットは動かない
  - 線引きが要る: apispec の文言は**描画のエラーボックスと `setoka check` の診断で共有**
    しているので、翻訳すると `--format json` の出力も言語で変わる
- **ドキュメントバージョニング** — 要否含め保留中
- **VS Code 拡張** — wasm プレビュー
  - 前提: `setoka-core` / `setoka-render` が 9 ファイルで `std::fs` に依存しており
    I/O 抽象化が要る
- **setoka 本体の crates.io 公開** — 汎用ライブラリ層は tankan・mikan まで公開済み
  - 障害: 名前 `yuzu`・`yuzu-core` が別プロジェクトに取得済み
  - 単一パッケージ化するか名称を再検討する必要がある（Phase 37 の決定事項）
  - 11-15: v0.21 Phase 94 で setoka へ改名すると決めた。crates.io の `setoka` は 11-15 の時点で空き。
    名前の問題は無くなるが、7 crate を出すか 1 つにまとめるかは残る（v0.21 では取らないと決めた = 94）

### レビューからの持ち越し（未着手）

レビューと見直しで見つかり、まだ着手していないもの（v0.20 Phase 93 で ROADMAP-history.md の
「レビューと見直しの記録」から写した）。**未着手の項目の状態はここで管理する**（片付けたら ~~取り消し線~~
と行き先を書く）。中身・対処案・判断の経緯は ROADMAP-history.md の同じ見出し。10-12 レビューの 4 つと
v0.10.1 レビューの 1 つは中身ごと、10-04 見直しと 10-07 見直しの分は見出しだけを写した
（10-07 の F1・F7 は上の「10-07 見直しからの候補」）。

- 10-04 見直し
  - ⬜ **権限と知識が 1 人に集中している**（工数: 小。v0.21 は PR を受け付けないので、main の保護と 2 人目の owner は扱わない）
  - ~~**0.x の互換性方針・CHANGELOG・アップグレードガイドが無い**~~ → v0.21 Phase 95
  - ⬜ **CI の穴**（工数: 中）
  - ~~**macOS バイナリが未署名・未公証**~~ → v0.21 Phase 96（説明だけ。署名・公証は判断点）
  - ~~**外部への露出が無い**~~ → v0.21 Phase 96・98（cargo-binstall・README・比較のページ）
  - ~~**外部からの貢献の受け口が無い**~~ → v0.21 Phase 97
  - ⬜ **長い関数**（工数: 中）
  - ⬜ **同じ処理が 2 か所にある**（工数: 小。v0.23 Phase 108。`escape_html` は v0.22 Phase 103 で setoka-render の写しを消して amanatsu の 1 実装にしたが、setoka-core の
    `markdown/mod.rs` に 1 つ残る。mermaid / math の言語の集合は今もコメントで同期させている）
  - ⬜ **性能の気になる点**（工数: 小〜中。どちらも未計測）
- 10-07 見直し
  - ⬜ **E1 公開 crate（tankan / mikan）の semver 上の危うさ**（工数: 小〜中）
  - ~~**E3 大事な経路のテストが無い**~~ → v0.22 Phase 101
  - ⬜ **E4 rel を `/` 区切りにする変換が `rel_to_slash` の外に 8 か所ある**（v0.23 Phase 110）
  - ⬜ **E5 静的ファイルを毎回全バイト読み、ページの sha256 を 1 ページ 4 回計算する**（v0.24 の候補。v0.23 の計画で v0.24 へ移した = 仮決め）
  - ~~**E6 ブラウザ側に自動の検証が無い**~~ → v0.22 Phase 100
  - ⬜ **F2 判断点が応答の無いまま推奨案で出荷されている**（工数: 小）
  - ⬜ **F3 レビューの記録が GitHub に残らない**（工数: 小）
  - ⬜ **F5 性能と、リリース後の不具合件数の推移を測っていない**（工数: 小〜中）
  - ~~**F6 文書と CI のゲートを二重・三重に管理している**~~ → v0.22 Phase 107（11-23。docs ゲートは `scripts/docs-gates.sh` の 1 か所にし、
    行番号の直書きをやめた。e2e は ci.yml に残した）
  - ⬜ **F8 docs の URL が独自ドメインと org 側のリポジトリに依存し、雛形に焼き込まれている**（工数: 小。v0.21 Phase 94 で URL は `/setoka/` に変えたが、利用者がユーザだけなので保有者・更新日の記録は扱わなかった）
- 10-12 レビュー（R6・R7・R8・R10）と v0.10.1 レビュー（中身ごと写したもの）

- ⬜ **R6 render_body_html（292 行）のノード生成と、記法ごとの組み替え**（v0.23 Phase 108。工数: 中）
  - `NodeHtmlBlock` の生成が 5 か所・`arena.alloc(AstNode::new(..))` が 4 か所、同じ 11 行の
    クロージャを 2 回定義している（`markdown/mod.rs`）。タブ・折りたたみ・用語集は解析側が
    tabs.rs / collapse.rs / glossary.rs にあるのに、組み替えだけが mod.rs にあり、記法の変更が
    2 ファイルにまたがる
  - 対処案（分割派と変更コスト派が合意）: ノード生成を 1 つの関数にまとめ、組み替えを各記法の
    モジュールへ移す。**適用の順序は本体の 1 関数に残す**（タブの判定は展開前・分量は展開後に
    数える・用語集は他の適用の後、の制約が記法をまたぐため）。本体は約 150 行になる見込み
  - あわせて CLAUDE.md の「`markdown/mod.rs` が comrak を触る唯一の場所」を「markdown/ 配下だけ」に
    直す（実際は 6 ファイル。**10-12 に確認**）
- ⬜ **R7 合成ページの `Page.src` は実在しないパスで、除外は否定のフィルタに頼っている**（v0.23 Phase 109）
  （工数: 中。**10-12 に確認**（件数））
  - `!p.is_generated()` / `!page.is_generated()` が 21 か所。書き込みや診断の経路を足すときに
    フィルタを書き忘れてもコンパイルが通る
  - 対処案: `PageOrigin { File(PathBuf), Generated(GeneratedKind) }` にし、ファイルのパスは
    `source_file() -> Option<&Path>` からしか取れなくする
- ~~**R8 出力先と原稿への書き込み口を clippy で縛る**~~ → v0.22 Phase 106（工数: 小。**10-12 に確認**（件数）。11-23 にルートの
  `clippy.toml` で縛った。書き込みに加えて削除と作成も禁じ、キャッシュ・setoka-index・setoka-render のディレクトリの作成と削除も
  kunenbo の口に替えた）
  - 「書き込みは `write_under` / `rewrite_source` だけ」はコメントで縛っている。今は守られている
    （テスト以外の `fs::write` / `remove_dir_all` は実装そのものと雛形の 4 か所だけ）
  - 対処案: `clippy.toml` の `disallowed-methods` で禁じ、4 か所だけ許す（前例は `main.rs` の
    `print_stdout`）
- ⬜ **R10 小さい整理**（工数: 各小。run 関数のテストと `lint --fix` の収束のループは v0.22 Phase 101、残りは v0.23 Phase 110。mikan の `make_excerpt` は Phase 114）
  - `build_once` から、テストの無い同義語の合成と丸め（IndexParams の組み立て）を純関数で出す
  - cli の run 関数（check・lint・search・dev・llms）にテストが 0 件。`lint --fix` の収束ループを
    `fmt.rs` のように純関数に出してテストする
  - `setoka_config::load` の保護対象の配列（`resolve.rs:84-89`）とエラー文言の match が別で、
    `_ =>` が `.setoka` の文言になっている。文言を配列に入れる
  - tankan: 前置き行（`%%{..}%%`・frontmatter・コメント）を飛ばす処理が 11 か所、SVG のルート要素が
    9 か所にコピーされ、frontmatter の `title:` の扱いが 4 通りに分かれた（引用符を外すのは packet
    だけ）。図種を足すたびにコピーが増える
  - mikan の `make_excerpt`（135 行）の窓を選ぶ処理を出し、総当たりの参照実装と比べるテストを書く
  - `BuildCache::meta(rel, source_hash)` は 2 つの `&str` を取り違えても通る（`SourceHash` 型。
    既知の「sha256 の 4 回計算」と一緒に）。`collect_sections` の `sections.last_mut().unwrap()`
    7 か所（11-28 は 6 か所）は状態の構造体にすれば消える。drafts・force の bool が 7 段そのまま渡る
- ⬜ **`.devcontainer/post-create.sh` の Claude Code インストーラ** — 取得した
  `install.sh` を検証せず bash へ渡している
  - 緩和されている点: **インストーラ自身がダウンロードしたバイナリを SHA-256 検証する**
    （バージョンごとの `manifest.json` の値と照合し、不一致なら削除して終了）
  - 残るギャップ: スクリプトの TOFU のみ
  - 固定を見送った理由: `install.sh` に公開チェックサムが無く、ベンダ更新のたびに
    devcontainer のビルドが壊れる
  - 着手するなら: バージョン指定（`bash -s -- <version>`）は可能なのでそこから

## これまでのリリース

各版の要約と Phase ごとの内訳は [ROADMAP-history.md](ROADMAP-history.md)。

- **v0.1**（Phase 1〜6）build / dev サーバ / 日本語検索 / llms.txt / tankan SSR / fmt・lint・check
- **v0.2**（Phase 7〜12）執筆表現 / 数式 / ページナビ / 検索のセクション単位化 / インクリメンタルビルド
- **v0.3**（Phase 13〜18）執筆の改善 / ページ Markdown の配信とコピー / 用語統一 lint
- **v0.4**（Phase 19〜23）表記ゆれの lint / 検索の同義語・誤字 / OpenAPI・JSON Schema の SSR
- **v0.5**（Phase 24〜29）tankan のスタイル構文 / コードブロックの索引
- **v0.6**（Phase 30〜35）検索の位置情報（フォーマット v3）/ フレーズ検索 / 並列化
- **v0.7**（Phase 36〜38）公開・配布の整備
- **v0.8**（Phase 39〜41）執筆機能の拡充
- **v0.9**（Phase 42〜45）執筆機能の拡充 第 2 弾
- **v0.10**（Phase 46〜49）実運用の質を上げる
- **v0.11**（Phase 50〜53）執筆機能の拡充 第 3 弾
- **v0.12**（Phase 54〜57）読む体験の完成
- **v0.13**（Phase 58〜61）lint の制御性
- **v0.14**（Phase 62〜63）設定基盤の刷新 = TOML 化
- **v0.15**（Phase 64〜67）正しさ・堅牢性
- **v0.16**（Phase 68〜71）kabosu の TOML 1.0 完全対応
- **v0.17**（Phase 72〜75）CLI の使い心地
- **v0.18**（Phase 76〜80）公開サイトの仕上げ
- **v0.19**（Phase 81〜86）配布物と第一印象
- **v0.20**（Phase 87〜93）最初の利用者が当たる所を直し、検証の穴を埋める
- **v0.21**（Phase 94〜99）外部への露出（名前を yuzu から setoka へ）
- **v0.22**（Phase 100〜107）単独で使える機能を crate に切り出し、検証の穴を埋める
