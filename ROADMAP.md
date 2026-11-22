# ロードマップ

setoka の開発計画（今の版の Phase・候補）と、これまでの版の 1 行の要約。**このファイルが Phase 状態の正**
（README には現在の版と概要だけを置く）。リリース済みの版の内訳と、レビュー・見直しの記録（中身と
判断の経緯）は [ROADMAP-history.md](ROADMAP-history.md)。レビュー・見直しの未着手の項目は、下の
「レビューからの持ち越し（未着手）」で状態を管理する。

## 現在: v0.22（Phase 100〜107）

**v0.21.0 まで公開済み**。kabosu 0.2.1 / tankan 0.2.2 / mikan 0.3.0 は crates.io で公開済み（setoka の
リリースとは非同期。手順は `publish-crate` スキル）。v0.21 の計画と各 Phase の記録（リリース後の作業を
当面しないと決めた記録を含む）は ROADMAP-history.md の「完了済み: v0.21（Phase 94〜99）の内訳」。

次は **v0.22**（テーマ「**単独で使える機能を crate に切り出し、検証の穴を埋める**」）を 8 つの Phase で
進める。11-17 に策定した。ユーザが「setoka-* の中で単独で使える機能は、名前を変えて出したい」と言い、
調べた候補 4 つ（API 仕様の HTML 化・日本語の表記ゆれの検査・開発サーバ・出力先への安全な書き込み）を
すべて選んだ。v0.22 は切り出しを中心にし（ユーザが決めた）、先に置いた依存の追随とコードの整理は
v0.23 の予定にした。

### v0.22 の方針

setoka の中の、ほかのツールでも使える機能を、tankan・mikan・kabosu と同じく setoka のほかの crate に依存しない
汎用ライブラリとして切り出し、crates.io へ出す。setoka はそれらを使う側になる。切り出しは内部の構造を
大きく動かすので、先にテストの穴（ブラウザ側・watch）を埋めてから行い、最後に docs ゲートを 1 つにまとめる。

以下は策定の詳細。

- 動機
  - 11-17 のユーザの要望。setoka-* を調べると、単独で使える機能が 5 つ見つかった（下の「現状の確認」）
  - v0.21 では、ブラウザでの確認をユーザに頼んだ。ブラウザ側と watch の自動の確認が無い（10-07 見直しの E6・E3）
  - ci.yml と verify スキルに docs ゲートを二重に書いている（10-07 見直しの F6）
- ゴール: 5 つの crate（shikuwasa・amanatsu・hebesu・jabara・kunenbo）が `crates/` にあり、setoka の
  ほかの crate に依存せず、`cargo package` が通る。setoka はそれらを使い、出力は切り出しの前と変わらない。
  ブラウザ側のスモークテストと watch の再ビルドの e2e が CI にある。docs ゲートが 1 か所にまとまっている
- 名前（11-17。ユーザが提案の組み合わせで決めた。どれも柑橘で、crates.io は空き・GitHub で同名の目立つ
  リポジトリは無い）

  | crate | 機能 | 切り出し元 |
  | --- | --- | --- |
  | shikuwasa（シークヮーサー） | YAML の安全な読み込み（別名の展開の上限・同じ名前のキーの検出・位置の換算・YAML 1.1 寄りのスカラーの解釈） | setoka-core の `yaml.rs` |
  | amanatsu（甘夏） | OpenAPI 3.x / Swagger 2.0 / JSON Schema を JS なしの静的 HTML にする | setoka-render の `apispec` |
  | hebesu（へべす） | 日本語の表記ゆれの検査（全角英数・半角カナ・長音符のゆれ・用語辞書）と自動修正 | setoka-core の `lint.rs` の一部 |
  | jabara（じゃばら） | 開発サーバ（静的ファイルの配信・ライブリロード・ファイルの監視・Host / Origin の検査・リンクの遮断） | setoka-server |
  | kunenbo（九年母） | 出力先への安全な書き込み（ルートの外・シンボリックリンクの検査・変わったときだけ書く・残ったファイルの掃除） | setoka-core の `output.rs` |

- 変わるもの: ワークスペースの crate の構成（5 つ増え、setoka-server は jabara に置き換わる）と依存の向き
  （CLAUDE.md の図）・CI（package の検査・ブラウザのテストのジョブ・e2e）・`publish-crate` スキル・
  第三者のライセンスの記録（新しい crate は自分の crate なので第三者ではない）。setoka の出力は変えない
  （スナップショットとキャッシュの指紋で確かめる）。`CACHE_FORMAT_VERSION` と MSRV は据え置く見込み
- Phase は「100 ブラウザ側の自動の確認 → 101 大事な経路のテスト → 102 shikuwasa → 103 amanatsu →
  104 hebesu → 105 jabara → 106 kunenbo → 107 docs ゲートの一本化と dogfooding」の順
  - 100・101 を先頭にするのは、切り出しで動かす経路（`fmt` / `lint --fix`・dev / preview・watch・検索の
    ブラウザ側）にテストが無いため
  - 102 を 103 より前に置くのは、amanatsu が shikuwasa を使うため（setoka-core の frontmatter も shikuwasa を使う）
  - 106 で、10-12 レビューの R8（出力先と原稿への書き込み口を clippy の `disallowed-methods` で縛る）も行う
  - 依存の PR #90（dependabot。saphyr 0.3.4 など 5 つ）は計画に入れず、着手の前に `dependency-update`
    スキルで片付ける（改名で Cargo.lock の crate 名が変わったので作り直しが要る。saphyr は shikuwasa の依存になる）
  - 着手時に判断点を決めてから実装する。応答が無いまま推奨案で進めた判断点は「仮決め」と PR と ROADMAP に書く
- 各 crate に共通する決めごと（tankan・mikan・kabosu にそろえる。着手時に変えてもよい）
  - `crates/<名前>/` に置き、setoka のほかの crate に依存しない。版はワークスペースと独立（0.1.0 から）。
    README は日本語、crates.io の description は英語。MSRV はワークスペースと同じ
  - 名前は柑橘のまま（11-17。ユーザが決めた）。名前だけでは用途が分からないので、crates.io の
    description・keywords・categories と README の 1 行目で用途を伝える（tankan・mikan・kabosu と同じ）
  - crates.io への公開は setoka のリリースと非同期で、ユーザが `publish-crate` スキルで行う（`cargo login` が要る）
  - ci.yml の「package」の検査（`LICENSE-MIT` / `LICENSE-APACHE` の同梱など）の対象に足す
- 入れないもの（v0.23 の予定にして候補に戻した）: vaporetto 0.7・mermaid の版・client 描画の Mermaid の書き間違いの
  検査・10-12 レビューの R6（本文の組み立て）・R7（合成ページの型）・R10 の残り・10-07 見直しの E4・E5・
  10-04 見直しの「同じ処理が 2 か所にある」。書き手の機能（非破壊 `init`・PR 検証 CI の雛形・Admonition の
  ラベル）・長い関数・性能・E1・F2・F3・F5・F8 なども候補に残す
- **リリース判定**（すべて満たしたらリリースする。どれもタグの前に確かめられる。「延期可」のものを v0.23 へ
  移したら、その行は判定から外してリリースノートに書く）
  - CI にブラウザ側のスモークテストのジョブがあり、通っている（100）
  - CI に watch の再ビルド（原稿を編集すると作り直す）と、dev / preview の HTTP の確認がある（101。
    setoka-cli の結合テストで、Windows・macOS でも動く。ci.yml のシェルの e2e を Windows へ移すのは延期可）
  - 5 つの crate が setoka のほかの crate に依存せず、それぞれの `cargo package` が CI で通る（102〜106）
  - setoka の出力が切り出しの前と変わらない（スナップショット・キャッシュの指紋・docs の dist）（102〜106）
  - clippy の `disallowed-methods` で、出力先と原稿への書き込みを kunenbo と `rewrite_source` の口に縛っている（106）
  - ci.yml と verify スキルが同じ docs ゲートのスクリプトを呼ぶ（107）
  - CLAUDE.md の依存の向きの図と `publish-crate` スキルが 5 つの crate を含む（107）
  - 仮決めのままの判断点が無い（残すならリリースノートに書く）
  - crates.io への公開は判定に入れない（setoka のリリースと非同期。setoka はワークスペースの path で使う）
- **非互換と注意に書く見込みのもの**: setoka の利用者から見た変化は無い見込み（出力が変わらないことを確かめる）。
  切り出しで文言・診断・HTML が変わったら書く。新しい crate はそれぞれの README と版で知らせる
- **ユーザの作業**

  | 作業 | Phase | 時期 |
  | --- | --- | --- |
  | テーマと範囲（切り出し中心）を計画の PR で確かめる。普段使っていて困っていることがあれば伝える | — | 計画の PR のレビュー |
  | 各 Phase の判断点を決める | 100〜107 | 着手時 |
  | 5 つの crate を crates.io へ publish する（`publish-crate` スキル） | 102〜106 | それぞれのマージ後（まとめてもよい） |

- **時間が足りなければ v0.23 へ移すもの**: 101 の ci.yml のシェルの e2e を Windows でも動かすこと（11-21 に
  仮決めで延期。Windows では結合テストが動く）/ 104 の CLI（hebesu を単独の
  コマンドとして出す部分）。100〜103・105〜107 と 104 のライブラリの部分は移さない

### 現状の確認（11-17）

- 切り出す機能（行数は 11-17）
  - YAML の読み込み: setoka-core の `yaml.rs`（707 行）。依存は saphyr と saphyr-parser だけ。frontmatter・
    未知キーの lint・API 仕様が使う
  - API 仕様の HTML 化: setoka-render の `apispec/render.rs`（1,927 行）と `apispec/mod.rs`（104 行）・
    `speccheck.rs`（180 行。setoka の診断に変える側）。setoka 側で使っているのは `highlight::escape_html` と
    `setoka_core::yaml::to_json` だけ。出力の文言は日本語（日本語を含む文字列が 70 か所）で、見た目は
    setoka のテーマの CSS（`api-spec` などのクラス）に頼る。外部の参照は `SpecFiles` の trait で読む
  - 日本語の表記ゆれの検査: setoka-core の `lint.rs`（1,324 行）の一部（`check_char_classes`・
    `check_katakana_choon`・`check_terms`・`apply_fixes` など）。`Page`・`Diagnostic`・Markdown の構文木に
    結びついていて、文字列を受ける関数として作り直す必要がある
  - 開発サーバ: setoka-server（1,623 行）。依存は axum・notify・tokio・tower-http だけで、setoka のほかの
    crate に依存していない
  - 出力先への書き込み: setoka-core の `output.rs`（1,184 行）。依存は std と same-file。ただし出力の記録
    （`OutputTracker`）は setoka の生成物の種類（`OutputKind` の `Site(SiteOutput)`）を持つ
- 検証
  - ブラウザ側の自動のテストは無い（テーマの JS は `crates/setoka-theme/assets/static/js/` の 13 ファイル）。
    docs は `backend = "ssr"` で、CI は docs の dist が `mermaid.min.js` を読むと落とすので、client 描画の図は
    docs では確かめられない（雛形は既定の client 描画）
  - ci.yml の watch の e2e は「何もしなければ再ビルドしない」側だけで、「編集すると作り直す」側が無い
  - ci.yml の `grep -q` は 198 個で、verify スキルにはそのうち 82 個を手で写している
- 依存の PR #90（11-15。saphyr 0.3.4・saphyr-parser 0.3.4・notify 8.2.2・toml 1.2.1・rust-embed 8.14.0）が
  開いたまま。作られた後に改名（PR #91）で Cargo.lock の crate 名が変わった

### 100 ブラウザ側の自動の確認 ✅

**概要**: docs サイトと雛形から作ったサイトの dist に対するブラウザのスモークテストを CI に足す（10-07
見直しの E6）。検索・配色の切替と保存・サイドバーの位置・キーボード操作・client 描画の図を、実際の
ブラウザで確かめる。

- 現状: 上の「現状の確認」の「検証」。v0.18 の Phase 80 と v0.21 の Phase 94 の確認は人手だった
- やること: `setoka preview` で docs の dist と、`setoka new` で作ったサイトの dist を配信し、ヘッドレスの
  ブラウザで 5〜10 本のスモークテストを動かすジョブを ci.yml に足す（client 描画の図と KaTeX は雛形の
  サイトで見る）。Node は CI のジョブの中だけで使い、リポジトリの依存にはしない
- 判断点: 使う道具（Playwright か、ほかのもの）と、テストのファイルの置き場 / 何を確かめるか / ジョブを
  PR ごとに動かすか
- 非互換と注意: なし（CI だけ）
- 決定（11-18。判断点への応答が無かったので推奨案で**仮決め**）
  - 道具は Playwright（@playwright/test 1.65.1。CI では Chromium だけを入れる）
  - 版は `tests/browser/package.json` の完全一致の指定で固定し、dependabot の npm で上げる。開発コンテナに
    Node が無く package-lock.json を作れないので、lock は置かない（@playwright/test が依存する playwright・
    playwright-core も完全一致で指定されている）。11-19 にコンテナへ Node を入れたので、lock は下のとおり
    決め直した
  - CI では PR ごとに動かす（ci.yml の `browser` ジョブ）
- 決定（11-19）: 手元でも動かす。開発コンテナに Node 26（26.13.1。LTS）を入れ、CI の `browser` ジョブも
  26 にそろえる。Playwright のブラウザ（Chromium）はイメージに焼かず、使うときに入れる
- 決定（11-19）: `tests/browser/package-lock.json` を置き、CI と手順は `npm ci` にする（3 つのパッケージの
  sha512 を照合してから入れる。Dockerfile のハッシュ照合・actions の SHA 固定と同じ扱い）
- やったこと
  - `tests/browser/`（`package.json`・`playwright.config.mjs`・`smoke.spec.mjs`・`README.md`）。テストは 8 本:
    docs サイトでヘッダーの検索欄・検索結果のページ・`/` キー・配色の切替（OS の設定 → ダーク → ライト →
    OS の設定）と再読み込み後の保存・サイドバーの位置の維持、雛形から作ったサイトで client 描画の Mermaid
    （「記法の例」の 9 つ）・KaTeX（3 つ）・コピーボタン。捕まえられなかった JS の例外が無いことも見る
  - ci.yml に `browser` ジョブ（docs と雛形のサイトをビルドし、`setoka preview` で配信して Playwright を
    動かす。失敗したら画面とトレースを artifact に残す）。`.github/dependabot.yml` に npm、`.gitignore` に
    テストの依存と結果を足した。verify スキルに手順 8、CLAUDE.md に注意を足した
  - テストが前提にする HTML（要素の id とクラス・数式と図の数・検索結果のページ）は、手元でビルドした dist で
    確かめた。ci.yml と dependabot.yml の YAML の構文は setoka の YAML の読み込みで確かめた
  - PR #99 でマージした（11-19。d887317）。CI の `browser` ジョブで 8 本とも通った（Node 22 での最初の実行）
  - 別の PR で、開発コンテナ（`.devcontainer/Dockerfile`）に Node 26 を入れた（版の固定＋nodejs.org の
    `SHASUMS256.txt` の sha256 と照合）。container.yml で `node --version`・`npm --version` を確かめる。ci.yml の
    `node-version` も 26（PR #101。CI の `browser` ジョブは Node 26 でも 8 本とも通った）
  - apple container でイメージを作り直し、コンテナ内で `npx playwright install --with-deps chromium` からテストまで
    動かして 8 本とも通った（11-19。ユーザが確認）。そのとき `npm install` で作った `package-lock.json` を置き、
    ci.yml・`tests/browser/README.md`・CLAUDE.md を `npm ci` にした
- 工数: 中

### 101 大事な経路のテスト ✅

**概要**: 切り出しで動かす経路にテストを足す（10-07 見直しの E3、10-12 レビューの R10 の一部）。watch の
再ビルド・dev / preview の HTTP・`lint --fix` と `--drafts`。

- 現状: 上の「現状の確認」の「検証」。cli の run 関数（check・lint・search・dev・llms）にテストが無い（R10）
- やること: ci.yml の e2e に「原稿を編集すると作り直す」watch の確認と、dev / preview で取ったページの確認を
  足す。`lint --fix` の収束のループを純関数に出してテストする。Windows でも CLI の e2e を動かすか（延期可）
- 判断点: watch の確認を e2e（シェル）にするか Rust のテストにするか / Windows の CLI の e2e を入れるか
- 非互換と注意: なし
- 決定（11-21。判断点への応答が無かったので推奨案で**仮決め**）
  - watch・dev / preview の HTTP・`lint --fix`・`--drafts` の確認は Rust の結合テスト（`crates/setoka-cli/tests/`）に
    する。ビルドした setoka を子プロセスで動かし、HTTP は std の `TcpStream` で取る（deny.toml が HTTP クライアントを
    禁止している）。`cargo test` に入るので、CI の test-cross で Windows・macOS でも動き、verify スキルへ手で写す
    必要もない
  - Windows で確かめるのはこの結合テストだけにする。ci.yml のシェルの e2e を Windows へ移すのは v0.23 へ延期する。
    watch のテストが macOS・Windows で不安定なら、やり直しで隠さず Linux だけに限って、ここに記録する
  - R10 の run 関数のテストは、check・lint・search・llms を 1 本ずつ（終了コードと出力の要点）。dev は watch の
    テストで見る。ci.yml の e2e は移さない
- やったこと
  - `lint --fix` の収束のループを `fix_until_stable`（`commands/lint.rs`）に出した。原稿の読み込み・診断・書き込みは
    引数で受け、テストで差し替える。テスト 6 本（1 回で止まる・連鎖を読み直して直す・上限で止める・適用 0 件と
    同じ結果では書かない・修正の無い診断では書かない・合成した用語集のページには書かない）。上限に達したときの
    動き（残りを違反として報告する）は変えない
  - 結合テスト `tests/commands.rs`（5 本）: check の終了コード 0 / 1 / 2・lint の検出と `--fix` と 2 回目は書かない
    （mtime）・search の結果と索引が無いときの 2・llms が dist 無しで出す・`build --drafts` で出た draft のページが
    次の通常のビルドで消える
  - 結合テスト `tests/serve.rs`（3 本）: `build --watch` と `dev` は原稿を編集すると dist を作り直す（dev は配信する
    ページも新しくなる）・dev のページにライブリロードの script がある・preview は dist を配信し、無いページは 404・
    preview のページにはライブリロードもオートリフレッシュも無い
  - ci.yml の「何もしなければ作り直さない」の確認（Linux）はそのまま残した
  - `cargo test` が結合テストのために `target/debug/setoka` を作り直すようになったので、CLAUDE.md と verify スキルの
    「更新しない」を直した。verify スキルのテストの手順は、TCP で待ち受ける `tests/serve.rs` をサンドボックスの外で
    動かすように分けた
  - PR #103 でマージした（11-21。c7e3e3c）。結合テストは CI の test-cross で macOS・Windows とも最初の実行から
    通った（watch のテストを Linux だけに限る必要は無かった）
  - レビュー指摘 2 件（1b00c69）: 起動の判定に、プロジェクトごとの目印を書いたページ（`/marker/`）を使う（空きを
    調べてから子プロセスがバインドするまでの間に、ほかのテストのサーバが同じポートで答えても起動と判断しない）。
    verify スキルの手順で、ローカルにサーバを立てる `commands/extlink.rs` の 3 本を含む `--bins` を
    サンドボックスの外の側へ移した
- 工数: 中

### 102 shikuwasa（YAML の安全な読み込み） ⬜

**概要**: setoka-core の `yaml.rs` を crate shikuwasa に切り出す。setoka-core（frontmatter・未知キーの lint）と
103 の amanatsu が使う。

- 現状: 上の「現状の確認」の「切り出す機能」。CLAUDE.md の「YAML の読み込み」の注意（saphyr の位置は文字数・
  別名の展開を数えない・`early_parse` 無効だとキーをスタイル込みで比べる）は、この実装が受け持っている
- やること: crate を作り、setoka-core から使う。公開する API（読み込み・JSON への変換・位置・エラー）を決める。
  CLAUDE.md の「YAML の読み込み」の節を shikuwasa を指すように直す。fuzz の対象に足すか決める
- 判断点: 公開 API の形（`serde_json::Value` を返す関数を入れるか＝serde_json に依存するか）/ fuzz の対象に足すか
- 非互換と注意: なし（frontmatter の読み方は変えない）。ただし CR だけで改行した原稿では、frontmatter のエラーと
  `frontmatter-unknown-key` の行と列が正しくなる（不具合の修正。以前は LF だけを数えて 1 行目を指していた。
  PR #108 のレビュー指摘）
- 決定（11-22。ユーザが推奨案で決めた）
  - 読んだ木は自前の型で返す（`Node`・`Kind`・`Scalar`・`Style`・`Tag`・`ScalarValue`。位置はバイトに直した
    `offset`）。saphyr の型は公開しない（saphyr を上げても shikuwasa の API は変わらない）
  - `to_json`（`serde_json::Value` へ写す）は既定で無効の feature `json` に入れる（serde_json の `preserve_order`）。
    使わない人の依存は saphyr と saphyr-parser だけ
  - エラーの文言は日本語のまま（tankan と同じ。README に書いた）
  - fuzz の対象に足す（`crates/shikuwasa/fuzz` の `parse`）
- やったこと
  - `crates/shikuwasa/`（0.1.0。`lib.rs`・`load.rs`・`scalar.rs`・`tests.rs`・README・LICENSE 2 つ）。中身は setoka-core の
    `yaml.rs` を移したもの（別名の展開の上限・同名のキー・コアタグ・スカラーの解釈・日本語の文言）に、自前の木への
    変換を足した。位置の変換は ASCII だけなら文字数のまま、ほかは文字の開始バイトの表を 1 回だけ作る（ノードごとに
    数え直さない）
  - setoka-core の `yaml.rs` を消し、frontmatter（`frontmatter.rs`）と frontmatter のエラーの位置（`markdown/mod.rs`）を
    shikuwasa に、setoka-render の API 仕様の読み込みを `shikuwasa::to_json` にした。setoka-core の依存から saphyr と
    saphyr-parser が外れた
  - テスト: yaml.rs のテストを移し、木の型・タグ付きの集合の位置・文字の途中を指す位置・feature `json` の写しを足した
    （feature 無しで 11 本・`json` 付きで 15 本・doc テスト 4 本）
  - fuzz: `crates/shikuwasa/fuzz`（`parse`。読み込み・位置が入力の中の文字の境界を指すこと・`to_json`）と種 3 つ
    （frontmatter・OpenAPI・別名とタグ）。fuzz.yml の対象に足した
  - ci.yml の package の検査（LICENSE の同梱を含む）に shikuwasa を足し、既定の feature（json 無し）でのテストを 1 行足した（ワークスペースのテストでは json 付きになる）。publish-crate・verify・release スキル、
    CLAUDE.md（依存の向きの図・凍結した設計判断・「1 実装で共有する箇所」の YAML の節・汎用ライブラリ）、docs の
    「開発者向け」（crate の一覧・依存の図・凍結した設計判断の表）を直した
  - crates.io の名前は 5 つ（shikuwasa・amanatsu・hebesu・jabara・kunenbo）とも空いている（11-22 に確かめた）
- 確かめたこと: `cargo test --workspace`（スナップショットとキャッシュの指紋を含めて通った = `CACHE_FORMAT_VERSION` は
  据え置き）・main と切り出し後の setoka で docs の dist（312 ファイル）がバイト単位で同じ・fuzz の対象の本体を
  スクラッチの crate で型検査して動かした（開発コンテナに C++ コンパイラが無く libfuzzer-sys をビルドできない）
- 未確認: レビュー指摘を直した後の fuzz の実行（次の定期実行か、publish の前の手動実行）
- レビュー指摘 2 件（PR #108）
  - [P2] saphyr が値を作れない循環した別名（`&a [*a]` の `*a`。BadValue）を値なしにしていた。frontmatter の
    `aliases: &a [*a]` が「実際: 値なし」になり、値なしは未指定として素通りする書き方もあった。`Kind::Unresolved` を
    足して区別し、切り出し前と同じ「実際: 値」の型の違い（終了コード 2）に戻した。`to_json` は従来どおり null
  - [P3] `line_col` が LF だけを数えていた（yaml.rs のときから）。YAML 1.2 と saphyr は CR だけの改行も認めるので、
    CR 区切りの文書のエラーが 1 行目を指していた。LF・CR・CRLF を数える（CRLF は 1 つ）
- fuzz: ブランチ（3e93073。上の 2 件を直す前）で fuzz.yml を手動実行し、6 対象とも成功（shikuwasa の `parse` は
  600 秒で 397 万回・panic、ハング、メモリ超過なし。ハーネスもここで初めてビルドした）
- 103 への注意（まだ確かめていない）: amanatsu は shikuwasa（json）に依存する。path の依存は crates.io に公開するときに
  版が要り、`cargo package -p amanatsu` は依存を crates.io から取ろうとする。103 の始めに、path の依存に `version` を
  付けて shikuwasa と amanatsu を 1 回の `cargo package` にまとめれば CI の toolchain で通るかを確かめる。通らなければ
  103 の package の検査を shikuwasa の公開まで外す（公開は取り消せないので、確かめる前に急いで publish しない）
- 工数: 小〜中

### 103 amanatsu（API 仕様の HTML 化） ⬜

**概要**: setoka-render の `apispec` を crate amanatsu に切り出す。OpenAPI 3.x / Swagger 2.0 / JSON Schema を、
JS なしの静的 HTML にする。

- 現状: 上の「現状の確認」の「切り出す機能」
- やること: crate を作り（shikuwasa に依存）、setoka-render から使う。`escape_html` は amanatsu が持つ。
  setoka 側の検査（`speccheck.rs`。setoka の診断に変える部分）は setoka-render に残す。HTML が切り出しの前と
  変わらないことをスナップショットで確かめる
- 判断点: 出力の文言（日本語のままにするか、文言を差し替えられるようにするか）/ 見た目の CSS を crate に
  同梱するか、クラス名を README に書くだけにするか / 公開 API（`SpecFiles` の trait・エラーの型）
- 非互換と注意: なし（HTML を変えない）
- 工数: 中

### 104 hebesu（日本語の表記ゆれの検査） ⬜

**概要**: setoka-core の `lint.rs` から、日本語の表記ゆれの検査（全角英数・半角カナ・長音符のゆれ・用語辞書）と
自動修正を crate hebesu に切り出す。文字列を受ける関数として作り直し、setoka-core は Markdown から取り出した
文字列を渡す側になる。

- 現状: 上の「現状の確認」の「切り出す機能」。抑制（`lintDisable`・行コメント）と診断の形は setoka-core の
  `suppress.rs`・`diagnostics.rs` にある
- やること: 文字列と位置を受けて、見つかったゆれ（位置・提案）を返す API を作る。長音符のゆれは複数の文書に
  またがる（多数派へそろえる）ので、その集計の形も決める。setoka-core の lint はそれを使い、診断・抑制・
  自動修正の結果が切り出しの前と同じであることを確かめる。単独のコマンド（Markdown を読んで検査する CLI）を
  出すか決める（延期可）
- 判断点: API の形（1 つの文字列ごとか、文書の集まりか）/ 単独の CLI を出すか（出すなら Markdown を読むために
  comrak に依存する）/ ルールの名前（setoka の `fullwidth-alphanumeric` などとそろえるか）
- 非互換と注意: なし（診断と自動修正の結果を変えない）
- 工数: 中〜大（CLI は延期可）

### 105 jabara（開発サーバ） ⬜

**概要**: setoka-server を crate jabara に置き換える。静的ファイルの配信・ライブリロード・ファイルの監視・
Host / Origin の検査・シンボリックリンクの遮断を、ほかの静的サイトの道具からも使えるようにする。

- 現状: 上の「現状の確認」の「切り出す機能」。CLAUDE.md の「watch と dev サーバ」の注意（出力ディレクトリの
  除外・イベントの種類で絞る・隠しディレクトリの判定は相対パス・Host / Origin の検査は IP を通す）は、この
  実装が受け持っている
- やること: setoka-server を jabara に改名し、setoka の都合で決めた部分（ライブリロードの経路 `/__livereload`・
  `base_path` の扱いなど）を設定で渡せるようにする。setoka-cli から使う。serve のテストは TCP をバインドするので、CI での
  動かし方は今のまま
- 判断点: 公開 API の形（`ServeOptions`・`watch` の引数）/ setoka の都合で決めた名前（`/__livereload` など）を
  どこまで設定にするか
- 非互換と注意: なし（dev / preview の動きを変えない）
- 工数: 小〜中

### 106 kunenbo（出力先への安全な書き込み） ⬜

**概要**: setoka-core の `output.rs` から、出力先への安全な書き込み（相対パスの検証・シンボリックリンクの
検査・変わったときだけ書く・一時ファイルからの置き換え・残ったファイルの掃除）を crate kunenbo に切り出す。
あわせて、書き込みの口を clippy で縛る（10-12 レビューの R8）。

- 現状: 上の「現状の確認」の「切り出す機能」
- やること: 汎用の部分を crate にし、setoka の生成物の種類を持つ出力の記録（`OutputTracker` と `OutputKind`）は
  setoka-core に残すか、種類を型の引数にして kunenbo に入れるかを決める。clippy の `disallowed-methods`
  （`clippy.toml`）で `fs::write` などを禁じ、kunenbo と `rewrite_source` の実装だけ許す
- 判断点: 出力の記録を kunenbo に入れるか / `rewrite_source`（原稿への書き込み）も kunenbo に入れるか
- 非互換と注意: なし
- 工数: 中

### 107 docs ゲートの一本化と dogfooding ⬜

**概要**: ci.yml と verify スキルに二重に書いている docs ゲートを、1 つのスクリプトにまとめて両方から呼ぶ
（10-07 見直しの F6）。v0.22 の切り出しを、文書（CLAUDE.md の依存の図・`publish-crate` スキル・docs の
開発者向けページ）に反映して通して確かめる。

- 現状: 上の「現状の確認」の「検証」。行番号や件数を文書に直書きしている箇所もある（F6）
- やること: `scripts/docs-gates.sh`（仮の名前）に docs ゲートをまとめ、ci.yml と verify スキルから呼ぶ。
  Bash 3.2 で動くように書く。CLAUDE.md の依存の向きの図・`publish-crate` スキルの表・docs の
  `development/index.md` の crate の図を 5 つの crate を含めて直す
- 判断点: スクリプトの名前と、e2e の確認も同じスクリプトにまとめるか
- 非互換と注意: なし
- 工数: 小〜中

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

- **client 描画の Mermaid の書き間違いを check で報告する**（v0.23 の予定。setoka-core は tankan に依存できないので、検査は setoka-render 側に置き `rules::CHECK_ONLY` に足す。mermaid の版の判断の後）（v0.19 Phase 85 から回した）— 今は
  既定の client では build も check も何も言わない。tankan のパースだけ通し、未対応
  （`is_unsupported`）以外の構文エラーを抑制できる warning で報告する案。tankan が未対応の
  書き方を誤りとして報告しないよう、切り分け方を決めてから扱う
- **mermaid 12 系への更新**（v0.23 の予定） — v0.18.1 で 11 系に留めた（12.x は tankan の互換対象の版と
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
- **vaporetto 0.7 系への更新**（v0.23 の予定。vaporetto に依存しモデルを同梱しているのは公開している mikan なので、mikan の版も上がる） — 0.7.0（10-24）はモデルの形式が変わり、同梱のモデル
  （vaporetto-models v0.5.0 の `bccwj-suw_c1.0`。0.6 の形式）を `model version mismatch` で読めない。
  モデルの差し替えと検索 wasm の作り直しが要るので、自動では上げず手動で行う（11-06。dependabot の
  #74・#75 を閉じ、`dependabot.yml` の ignore に足した）
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
  - ⬜ **同じ処理が 2 か所にある**（工数: 小。v0.23 の予定。`escape_html` は v0.22 の amanatsu の切り出しで 1 つ減る見込み）
  - ⬜ **性能の気になる点**（工数: 小〜中。どちらも未計測）
- 10-07 見直し
  - ⬜ **E1 公開 crate（tankan / mikan）の semver 上の危うさ**（工数: 小〜中）
  - ~~**E3 大事な経路のテストが無い**~~ → v0.22 Phase 101
  - ⬜ **E4 rel を `/` 区切りにする変換が `rel_to_slash` の外に 8 か所ある**（v0.23 の予定）
  - ⬜ **E5 静的ファイルを毎回全バイト読み、ページの sha256 を 1 ページ 4 回計算する**（v0.23 の予定）
  - ~~**E6 ブラウザ側に自動の検証が無い**~~ → v0.22 Phase 100
  - ⬜ **F2 判断点が応答の無いまま推奨案で出荷されている**（工数: 小）
  - ⬜ **F3 レビューの記録が GitHub に残らない**（工数: 小）
  - ⬜ **F5 性能と、リリース後の不具合件数の推移を測っていない**（工数: 小〜中）
  - ~~**F6 文書と CI のゲートを二重・三重に管理している**~~ → v0.22 Phase 107
  - ⬜ **F8 docs の URL が独自ドメインと org 側のリポジトリに依存し、雛形に焼き込まれている**（工数: 小。v0.21 Phase 94 で URL は `/setoka/` に変えたが、利用者がユーザだけなので保有者・更新日の記録は扱わなかった）
- 10-12 レビュー（R6・R7・R8・R10）と v0.10.1 レビュー（中身ごと写したもの）

- ⬜ **R6 render_body_html（292 行）のノード生成と、記法ごとの組み替え**（v0.23 の予定。工数: 中）
  - `NodeHtmlBlock` の生成が 5 か所・`arena.alloc(AstNode::new(..))` が 4 か所、同じ 11 行の
    クロージャを 2 回定義している（`markdown/mod.rs`）。タブ・折りたたみ・用語集は解析側が
    tabs.rs / collapse.rs / glossary.rs にあるのに、組み替えだけが mod.rs にあり、記法の変更が
    2 ファイルにまたがる
  - 対処案（分割派と変更コスト派が合意）: ノード生成を 1 つの関数にまとめ、組み替えを各記法の
    モジュールへ移す。**適用の順序は本体の 1 関数に残す**（タブの判定は展開前・分量は展開後に
    数える・用語集は他の適用の後、の制約が記法をまたぐため）。本体は約 150 行になる見込み
  - あわせて CLAUDE.md の「`markdown/mod.rs` が comrak を触る唯一の場所」を「markdown/ 配下だけ」に
    直す（実際は 6 ファイル。**10-12 に確認**）
- ⬜ **R7 合成ページの `Page.src` は実在しないパスで、除外は否定のフィルタに頼っている**（v0.23 の予定）
  （工数: 中。**10-12 に確認**（件数））
  - `!p.is_generated()` / `!page.is_generated()` が 21 か所。書き込みや診断の経路を足すときに
    フィルタを書き忘れてもコンパイルが通る
  - 対処案: `PageOrigin { File(PathBuf), Generated(GeneratedKind) }` にし、ファイルのパスは
    `source_file() -> Option<&Path>` からしか取れなくする
- ~~**R8 出力先と原稿への書き込み口を clippy で縛る**~~ → v0.22 Phase 106（工数: 小。**10-12 に確認**（件数））
  - 「書き込みは `write_under` / `rewrite_source` だけ」はコメントで縛っている。今は守られている
    （テスト以外の `fs::write` / `remove_dir_all` は実装そのものと雛形の 4 か所だけ）
  - 対処案: `clippy.toml` の `disallowed-methods` で禁じ、4 か所だけ許す（前例は `main.rs` の
    `print_stdout`）
- ⬜ **R10 小さい整理**（工数: 各小。run 関数のテストと `lint --fix` の収束のループは v0.22 Phase 101、残りは v0.23 の予定）
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
    7 か所は状態の構造体にすれば消える。drafts・force の bool が 7 段そのまま渡る
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
