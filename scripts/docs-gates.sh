#!/usr/bin/env bash
# このリポジトリ自身のドキュメントサイト（docs/）の検証。setoka check と build を通し、機能ごとの配信ゲート
# （生成物の grep）を照合する。ci.yml の docs ステップと verify スキルの 5 節が呼ぶ（v0.22 Phase 107。
# 10-07 見直しの F6 = 以前は ci.yml と verify スキルに同じゲートを二重に書いていた）。
#
#   scripts/docs-gates.sh [setoka のバイナリ]
#
# バイナリを省くと ${CARGO_TARGET_DIR:-target}/debug/setoka（先に cargo build -p setoka-cli しておく）。
# リポジトリのルートで --root docs として実行する（cd docs しない。Phase 75 の dogfooding）。出力は docs/dist/。
# 最後に --base-url /setoka/ でも作るので、終わったときの docs/dist は既定の base で作り直してある。
# jq が要る。終了コード: 0 = すべて通った / 0 以外 = どこかのゲートで止まった（止まった行を出す）
#
# ゲートの足し方: 新しい機能・原稿の節を足したら、ここに 1 行足す（ci.yml・verify スキルには書かない）。
# 1 行に 1 つのゲートにし、否定のゲートは `if grep …; then echo …; exit 1; fi` の形にする。
# **ゲートを素通りさせる書き方は使えない**（bash -e は `A && B` の A・`A || B` の A・`!` で反転したコマンドの失敗では
# 止まらず、`|| true` や `set +e` は失敗を消す。ci.yml の docs ステップにあった `grep -q A && grep -q B` の 10 行は
# A がゲートになっていなかった）。起動時にこのファイル自身を調べ、コメント以外の行の && と ||・! で始まる文・
# set +e があれば止まる。条件を繋ぐときは if を入れ子にし、検索する文字列に && や || が要るときは [&][&] のように書く。
#
# macOS 標準の Bash 3.2 でも動くように書く（mapfile・連想配列・${var,,} 等の Bash 4 以降の機能を使わない。
# 日本語が続く変数は ${var} と囲む。verify スキルからホストで呼ぶ）
set -euo pipefail

# このファイル自身に、ゲートを素通りさせる書き方が無いか（上の決まり。この行の正規表現は、文字クラスで書いて
# 自分自身に一致しないようにしてある）
bypass="$(awk '/^[[:space:]]*#/ { next } /&[&]|[|][|]|^[[:space:]]*![[:space:]]|set[[:space:]]+[+]e/ { print FNR ": " $0 }' "$0")"
if [ -n "$bypass" ]; then
  echo "scripts/docs-gates.sh にゲートを素通りさせる書き方があります（AND・OR の演算子 / ! で始まる文 / errexit を止める set。1 行に 1 つのゲートにし、条件は if を入れ子にする）:" >&2
  echo "$bypass" >&2
  exit 2
fi

# 日本語の文字列を文字単位で扱うため、実在する UTF-8 のロケールを使う（scripts/check-doc-refs.sh と同じ）
for loc in C.UTF-8 C.utf8 en_US.UTF-8 en_US.utf8; do
  if locale -a 2>/dev/null | grep -qx "$loc"; then
    export LC_ALL="$loc"
    break
  fi
done

# リポジトリのルートへ移る（失敗したら set -e で止まる）
cd "$(dirname "$0")/.."
SETOKA="${1:-${CARGO_TARGET_DIR:-target}/debug/setoka}"
if [ ! -x "$SETOKA" ]; then
  echo "setoka のバイナリがありません: ${SETOKA}（先に cargo build -p setoka-cli する）" >&2
  exit 2
fi
if ! command -v jq >/dev/null; then
  echo "jq がありません（検索の順位の照合に使う）" >&2
  exit 2
fi

# どのゲートで止まったかを出す（grep -q は何も言わずに 1 で終わるため）
trap 'echo "docs のゲートで止まりました（scripts/docs-gates.sh の ${LINENO} 行目）: ${BASH_COMMAND}" >&2' ERR

# 診断は、GitHub Actions の中では注釈で出す（Phase 46）。パスは GITHUB_WORKSPACE からの
# 相対へ自動で付け替わるので PR の diff 行に対応する（cwd に依存しない）。
# 注釈の画面表示は 1 ステップ 10 件までで、残りはログに出る。手元では人が読む形で出す
if [ "${GITHUB_ACTIONS:-}" = "true" ]; then
  "$SETOKA" check --root docs --format github
else
  "$SETOKA" check --root docs
fi
"$SETOKA" build --root docs
test -f docs/dist/index.html
test -f docs/dist/_search/manifest.json
# エイリアス（guide/quality の旧 URL）にリダイレクト HTML が生成される
grep -q 'http-equiv="refresh"' docs/dist/guide/lint/index.html
# コンテンツインクルード: setoka.toml の引用が展開され、描画したキャプション（`setoka.toml:行範囲`）の範囲が原稿の `lines=` と
# 同じ（行番号の直書きをやめた代わりに、原稿から読んで照合する。キャプションの範囲を見ないと、描画が原稿と食い違っても
# 通った。PR #114 のレビュー指摘）。原稿は、`file="setoka.toml"` を含むフェンスの行（実際の引用と、書き方の例）の `lines=` を、
# 属性の順に依らずに読む。引用と例の範囲がそろっていること・その範囲が setoka.toml の `[markdown]` とその下の表を漏れなく・
# 余分なく指していることは、setoka の情報文字列の読み取りと kabosu を使う setoka-core のテスト
# （crates/setoka-core/tests/docs_excerpt_test.rs。cargo test で動く）が確かめる（v0.22 Phase 107。シェルで TOML を読むと、
# 複数行の文字列・エスケープしたキーなどで見逃した）
ranges="$(awk '/^[[:space:]]*(```|~~~)/ { if ($0 ~ /file=["\047]?setoka\.toml["\047]?([[:space:]]|$)/) { if (match($0, /lines=[0-9]+-[0-9]+/)) print substr($0, RSTART + 6, RLENGTH - 6) } }' docs/content/guide/code-and-math.md | sort -u)"
count="$(printf '%s\n' "$ranges" | awk 'NF { n++ } END { print n + 0 }')"
if [ "$count" -ne 1 ]; then
  echo "code-and-math.md の setoka.toml の引用（lines=）が読めないか、範囲がそろっていません: ${ranges}" >&2
  exit 1
fi
captions="$(awk '{ s = $0; while (match(s, /<figcaption>setoka\.toml:[0-9]+-[0-9]+<\/figcaption>/)) { t = substr(s, RSTART, RLENGTH); sub(/^<figcaption>setoka\.toml:/, "", t); sub(/<\/figcaption>$/, "", t); print t; s = substr(s, RSTART + RLENGTH) } }' docs/dist/guide/code-and-math/index.html | sort -u)"
if [ "$captions" != "$ranges" ]; then
  echo "code-and-math のページの setoka.toml の引用のキャプション（${captions:-無し}）が原稿の lines=${ranges} と違います" >&2
  exit 1
fi
# 図表番号: キャプションの採番と参照リンクの自動補完
grep -q 'id="fig:deps"' docs/dist/development/index.html
grep -q '<a href="#fig:deps">図 1</a>' docs/dist/development/index.html
# 折りたたみ Admonition: details/summary で描画される
grep -q '<details class="markdown-alert markdown-alert-tip">' docs/dist/guide/writing/index.html
# 折りたたみ内へのアンカージャンプを開く JS が配信される
grep -q 'js/details-target.js' docs/dist/guide/writing/index.html
# サイドバーのスクロール位置維持（base.jinja のインライン script）が配信されている
grep -q 'setoka-sidebar-scroll' docs/dist/index.html
# タブ / コードグループ（Phase 50）: radio + label のマークアップが出ている
grep -q 'class="tab-label"' docs/dist/guide/code-and-math/index.html
# Markdown 断片（Phase 51）: docs/snippets/ の断片が展開された散文で出ている
grep -q '取り込まれた Markdown 断片です' docs/dist/guide/writing/index.html
# 用語集（Phase 52）: 本文の略語が辞書の説明つきで <abbr> 化されている
grep -q '<abbr title="Server-Side Rendering' docs/dist/guide/writing/index.html
# 用語集ページが自動生成され、用語ごとのアンカーが振られている
grep -q 'id="ssr"' docs/dist/glossary/index.html
# 合成ページが nav 経路に乗っている（サイドバーに出る）
grep -q 'href="/glossary/"' docs/dist/index.html
# 用語集ページ自身は abbr 化しない（説明文の中で用語が光る循環を防ぐ）
if grep -q '<abbr' docs/dist/glossary/index.html; then
  echo "用語集ページ自身が abbr 化された" >&2
  exit 1
fi
# 約物に隣接した強調（Phase 53 の cjk_friendly_emphasis）
grep -q '<strong>「重要」</strong>' docs/dist/guide/writing/index.html
grep -q '<strong>この文は重要です。</strong>但し' docs/dist/guide/writing/index.html
# 定義リスト（Phase 53 の description_lists）
grep -q '<dl>' docs/dist/guide/writing/index.html
# 検索の絞り込み: インデックスにナビ由来の区分が焼き込まれている
grep -q '"groups"' docs/dist/_search/manifest.json
grep -q '"docGroups"' docs/dist/_search/manifest.json
# エンジン側で絞り込めて件数も絞り込み後になる（ブラウザ操作は CI で
# 検証できないので、同一エンジンを通る CLI で確かめる）
"$SETOKA" search --root docs --section 開発者向け "キャッシュ" | grep -q '/development/'
# Phase 89（リリース判定）: 「設定」で設定のページが上位 3 件に入る。以前は同義語の例を
# 書いた節が 1 位で、設定のページは 14 位だった。ネイティブの setoka search はブラウザと
# 同じエンジンを通る（wasm の作り直し忘れは search wasm inputs の照合が見る）
"$SETOKA" search --root docs --format json --limit 3 "設定" \
  | jq -e 'any(.[]; .url | startswith("reference/config/"))' >/dev/null
# 絞り込み UI の配線ゲート（機能自体はブラウザ操作なので CI では検証できない。
# Phase 54 でチップは結果ページ search-page.js へ移動した）
grep -q 'search-filter-chip' docs/dist/_assets/js/search-page.js
grep -q 'composedPath' docs/dist/_assets/js/search-ui.js
# 診断ルールのリファレンス（Phase 46）が配信されている
grep -q 'broken-anchor' docs/dist/reference/rules/index.html
# 出力先・URL の安全化（v0.10.1）で足したルールが原稿に載っている
grep -q 'route-conflict' docs/dist/reference/rules/index.html
grep -q 'unsafe-page-path' docs/dist/reference/rules/index.html
# URL のパーセントエンコード（Phase 64）が原稿に載っている
grep -q 'パーセントエンコード' docs/dist/guide/writing/index.html
grep -q '%E8%A8%AD%E8%A8%88/%E6%A6%82%20%E8%A6%81' docs/dist/guide/writing/index.html
# 配信とテーマ契約（Phase 65）: テーマ上書きの追随責任と配信のリンク遮断が
# 原稿に載り、docs（ハイライト有効）では syntect.css が <link> される
# （テーマ・ダークモード・印刷は Phase 84 で「見た目の調整」へ移した）
grep -q '追随する' docs/dist/guide/appearance/index.html
grep -q 'シンボリックリンクを辿りません' docs/dist/reference/cli/index.html
grep -q 'css/syntect.css' docs/dist/index.html
test -f docs/dist/_assets/css/syntect.css
# 外部リンク検査（Phase 66）が原稿に載っている
grep -q 'external-link-broken' docs/dist/reference/rules/index.html
grep -q -- '--external-links' docs/dist/reference/cli/index.html
grep -q 'ネットワーク I/O' docs/dist/development/index.html
# ページ単位の lint 抑制（Phase 58）が原稿に載っている
grep -q 'unused-lint-suppression' docs/dist/reference/rules/index.html
grep -q 'lintDisable' docs/dist/guide/quality/index.html
# 行単位の lint 抑制（Phase 59）が原稿に載っている
grep -q 'setoka-lint-disable-next-line' docs/dist/reference/rules/index.html
grep -q 'setoka-lint-disable-next-line' docs/dist/guide/quality/index.html
# 全ルールの enable/disable（Phase 60）が原稿に載っている
grep -q 'プロジェクト全体の無効化' docs/dist/reference/rules/index.html
grep -q 'プロジェクト全体の無効化' docs/dist/guide/quality/index.html
# 検索の追加読み込み（Phase 49 → Phase 54 で結果ページへ移動）の JS が
# 埋め込み経由で配信されている（機能自体はブラウザ操作なので CI では
# 検証できない。配線ゲート）
grep -q 'search-page-more' docs/dist/_assets/js/search-page.js
# 検索結果ページ（Phase 54）: 専用ページが生成され、集約からは除外される
test -f docs/dist/search/index.html
grep -q 'id="setoka-search-page"' docs/dist/search/index.html
grep -q 'data-page-size' docs/dist/search/index.html
# ドロップダウン → 結果ページへの導線（全ページに data-search-page が配られる）
grep -q 'data-search-page' docs/dist/index.html
# llms.txt に検索結果ページが載らない（guide/search.md は実ページなので残る）
if grep -q '\[検索\](' docs/dist/llms.txt; then
  echo "llms.txt に検索結果ページが載った" >&2
  exit 1
fi
# ページ単位 .md も出さない
if test -f docs/dist/search.md; then
  echo "検索結果ページのページ単位 .md が出た" >&2
  exit 1
fi
# 公開範囲の制御（Phase 83）: 社内で公開するページと、.md を止める設定・Host の検査の説明
test -f docs/dist/guide/internal/index.html
grep -q 'page_md' docs/dist/guide/llms/index.html
grep -q 'allowed_hosts' docs/dist/reference/config/index.html
# ナビと目次の規模対応（Phase 56）: 現在セクションだけ開いた折りたたみ・
# 他セクションは閉じる・TOC の nav 化・scrollspy 基準線の実測化
grep -q '<details class="nav-section" open>' docs/dist/guide/writing/index.html
grep -q '<details class="nav-section">' docs/dist/guide/writing/index.html
grep -q '<nav aria-label="このページの目次">' docs/dist/guide/writing/index.html
grep -q 'scrollMarginTop' docs/dist/_assets/js/scrollspy.js
# 印刷 / PDF 対応（Phase 55）: print CSS・ダーク定義の画面専用化・
# 印刷前に閉じた details を開く JS が配信されている
grep -q '@media print' docs/dist/_assets/css/theme.css
grep -q '@media screen' docs/dist/_assets/css/syntect.css
grep -q 'beforeprint' docs/dist/_assets/js/details-target.js
# SSR 図のモバイル対応（Phase 61）: 縮小せず figure 内で横スクロール
grep -A 3 'figure.mermaid-ssr {' docs/dist/_assets/css/theme.css | grep -q 'overflow-x: auto'
# ビルドの進捗ログ（RUST_LOG の文書化）が CLI リファレンスに載っている
grep -q 'RUST_LOG' docs/dist/reference/cli/index.html
# グローバルフラグ --root（Phase 72）が CLI リファレンスと設定リファレンスに載っている
grep -q -- '--root' docs/dist/reference/cli/index.html
grep -q -- '--root' docs/dist/reference/config/index.html
# 静粛モード -q / -v と search --format（Phase 73）が CLI リファレンスに載っている
grep -q -- '--quiet' docs/dist/reference/cli/index.html
grep -q '検索結果の JSON 配列' docs/dist/reference/cli/index.html
# シェル補完（Phase 74）の節が CLI リファレンスに載っている
# （コードブロック内の文字列はハイライトの span で分断されるので見出し id で照合する）
grep -q 'id="シェル補完"' docs/dist/reference/cli/index.html
# <head> の共有カード（Phase 76）: URL に依らないタグは常に出る。
# canonical / og:url はフル URL のときだけなので、ここ（base_url 無し）では出ない
grep -q '<meta property="og:title"' docs/dist/index.html
grep -q '<meta name="twitter:card" content="summary">' docs/dist/index.html
if grep -q 'rel="canonical"' docs/dist/index.html; then
  echo "base_url がフル URL でないのに canonical が出た" >&2
  exit 1
fi
grep -q 'id="共有カードogpと-canonical"' docs/dist/guide/deploy/index.html
# Phase 77: frontmatter の読み違いのルールと、雛形 deploy.yml の版固定の説明
grep -q 'frontmatter-unrecognized' docs/dist/reference/rules/index.html
grep -q 'setoka と同じ版に' docs/dist/guide/deploy/index.html
# Phase 78: 見出しのパーマリンク。id は見出し自身に付き、リンクは末尾に
# aria-label 付き（aria-hidden の付いた旧形式が残っていたら失敗）。
# CSS は visibility: hidden でなく、フォーカスで # を出す
grep -q '<h2 id="frontmatter">frontmatter<a class="anchor" href="#frontmatter" aria-label="「frontmatter」へのリンク">' docs/dist/guide/writing/index.html
if grep -rl 'aria-hidden="true" class="anchor"' docs/dist/ --include="*.html"; then
  echo "aria-hidden の付いた見出しアンカーが残っている（上記ページ）" >&2
  exit 1
fi
grep -q '.anchor:focus-visible' docs/dist/_assets/css/theme.css
# Phase 78: 読了時間と文字数（既定で表示）
grep -q '<p class="page-reading">約 [0-9]* 分で読めます' docs/dist/guide/writing/index.html
# Phase 79: OS ダーク追従。toggle（docs の設定）では data-theme を付けず、
# theme.css と syntect.css に prefers-color-scheme の系統がある
grep -q '<html lang="ja">' docs/dist/index.html
grep -q 'prefers-color-scheme: dark' docs/dist/_assets/css/theme.css
grep -q 'prefers-color-scheme: dark' docs/dist/_assets/css/syntect.css
grep -q 'id="ダークモード"' docs/dist/guide/appearance/index.html
# Phase 80: 共有カードの画像（site.image）が配信物に入っている
# （og:image タグ自体はフル URL の公開ビルドでだけ出るので、ここではファイルを見る）
test -f docs/dist/images/og.png
# Phase 84: 目次の見直し（インストール・見た目・FAQ を独立したページに）と
# 図の選び方・既存のフォルダで始める手順・Issues への導線
grep -q 'id="既存のフォルダで始める"' docs/dist/guide/install/index.html
grep -q 'id="dist-はサーバで配信する"' docs/dist/guide/install/index.html
test -f docs/dist/guide/appearance/index.html
test -f docs/dist/guide/faq/index.html
grep -q 'id="client-と-ssr-の選び方"' docs/dist/guide/diagrams/index.html
grep -q 'github.com/ai-implementer/setoka/issues' docs/dist/index.html
grep -q 'github.com/ai-implementer/setoka/issues' docs/dist/guide/faq/index.html
# Phase 85: build の報告・check --no-fmt・大文字小文字だけが違う URL・ログの書式・
# fmt が変える書き方の一覧が原稿に載っている
grep -q -- '--no-fmt' docs/dist/reference/cli/index.html
grep -q 'NO_COLOR' docs/dist/reference/cli/index.html
grep -q 'route-case-conflict' docs/dist/reference/rules/index.html
grep -q '取り込みの失敗' docs/dist/guide/quality/index.html
grep -q 'end list' docs/dist/guide/quality/index.html
# Phase 86: docs サイト（dogfooding）にもライセンス表記が出て、vendor の開発用の記録は
# 出ない。利用者のサイトに出るライセンス表記の説明が原稿に載っている
test -s docs/dist/_assets/vendor/THIRD-PARTY-LICENSES.txt
test -s docs/dist/_search/THIRD-PARTY-LICENSES.txt
if [ -e docs/dist/_assets/vendor/README.md ]; then
  echo "docs の dist に vendor の開発用の記録（README.md）が出た" >&2
  exit 1
fi
grep -q 'id="第三者のソフトウェアのライセンス表記"' docs/dist/guide/internal/index.html
# Phase 88: frontmatter の YAML の読み方（YAML 1.2・YAML 1.1 の書き方はエラー）の説明
grep -q 'id="yaml-の書き方"' docs/dist/guide/writing/index.html
# Phase 89: 検索の並び順（タイトル一致の優遇）と search: false の説明
grep -q 'id="結果の並び順"' docs/dist/guide/search/index.html
grep -q 'id="検索から外す"' docs/dist/guide/search/index.html
# Phase 90: favicon・フッター・ヘッダーのリンクの説明
grep -q 'id="faviconフッターヘッダーのリンク"' docs/dist/guide/appearance/index.html
# Phase 91: content の外を指すリンクのルール・build の警告と要約の行の説明
grep -q 'link-outside-content' docs/dist/reference/rules/index.html
grep -q 'link-outside-content' docs/dist/guide/writing/index.html
grep -q '出力: ページ' docs/dist/guide/quality/index.html
grep -q '利用者のファイルを優先' docs/dist/guide/quality/index.html
# Phase 91: 他のツールとの違いのページ・雛形 deploy.yml の全文・PR 検証の例と CI の例の --no-fmt
grep -q 'id="他のツールとの違い"' docs/dist/guide/migrate/index.html
grep -q 'id="生-html-と画像の大きさ"' docs/dist/guide/migrate/index.html
grep -q '<figcaption>snippets/deploy.yml</figcaption>' docs/dist/guide/deploy/index.html
grep -q 'id="ワークフローの全文"' docs/dist/guide/deploy/index.html
grep -q '<figcaption>snippets/pr-check.yml</figcaption>' docs/dist/guide/quality/index.html
grep -q -- '--no-fmt --format github' docs/dist/guide/quality/index.html
# v0.21 Phase 95: 互換性の約束と更新の手順のページ
grep -q 'id="約束の対象"' docs/dist/reference/compatibility/index.html
grep -q 'id="新しい版に上げる手順"' docs/dist/reference/compatibility/index.html
# v0.21 Phase 96: インストールの 3 通り（cargo-binstall の節）と、macOS の Gatekeeper の FAQ
grep -q 'id="cargo-binstall-で入れる"' docs/dist/guide/install/index.html
grep -q '第三者がビルドしたバイナリ（quickinstall）や' docs/dist/guide/install/index.html
grep -q 'id="macos-で開いていませんと止められる"' docs/dist/guide/faq/index.html
# v0.21 Phase 97: 問い合わせの受け口（PR は受け付けない・CONTRIBUTING と issue のテンプレート）
grep -q 'pull request は受け付けていません' docs/dist/guide/faq/index.html
test -s CONTRIBUTING.md
test -s .github/ISSUE_TEMPLATE/config.yml
# v0.21 Phase 98: mdBook との比較と選び方のページ（比べた版を書く）
grep -q 'id="どちらが合うか"' docs/dist/guide/compare/index.html
grep -q 'mdBook v0.5.4' docs/dist/guide/compare/index.html
# 設定例は TOML で書く（JSON / YAML 風の `markdown.mermaid.backend: "ssr"` や
# `"abbr": false` はコピーすると設定エラーになる）。原稿・README・雛形・スキルを見る
if grep -rnE '`[a-z_]+(\.[a-z_]+)+: |`"[a-z_]+": |backend: ["]' docs/content README.md crates/setoka-cli/scaffold/*.md .claude/skills/; then
  echo "JSON / YAML 風の設定例が残っている（上記。TOML の書き方に直す）" >&2
  exit 1
fi
# mermaid backend: ssr のサイトなので、フォールバック（= mermaid.js の
# script タグ）が出たら tankan の回帰として失敗させる
# （本文中の文字列と区別するため src 属性の形で照合する）
if grep -rlE 'src="[^"]*vendor/mermaid\.min\.js"' docs/dist/ --include="*.html"; then
  echo "docs で SSR フォールバックが発生している（上記ページ）" >&2
  exit 1
fi
# v0.22 Phase 107（dogfooding）: crates.io に出す crate（Cargo.toml に publish = false が無いもの）が、docs の
# 「開発者向け」の crate の一覧・CLAUDE.md の汎用ライブラリの一覧・publish-crate スキルの表・ci.yml の package の
# 検査のすべてに載っている（crate を切り出したときの直し忘れを見る。名前は区切りまで照合する = mikan と mikan-wasm を
# 取り違えない）
for manifest in crates/*/Cargo.toml; do
  if grep -q '^publish = false' "$manifest"; then
    continue
  fi
  crate="$(basename "$(dirname "$manifest")")"
  if ! grep -qE "[├└]─ ${crate} " docs/dist/development/index.html; then
    echo "docs の「開発者向け」の crate の一覧に ${crate} が無い" >&2
    exit 1
  fi
  if ! grep -E '^tankan・.*は他の setoka crate 非依存の汎用ライブラリ$' CLAUDE.md | grep -qE "(^|・)${crate}(・| は)"; then
    echo "CLAUDE.md の汎用ライブラリの一覧に ${crate} が無い" >&2
    exit 1
  fi
  if ! grep -qF "| ${crate} |" .claude/skills/publish-crate/SKILL.md; then
    echo "publish-crate スキルの表に ${crate} が無い" >&2
    exit 1
  fi
  if ! grep -E '^ +cargo package --locked -p ' .github/workflows/ci.yml | grep -qE -- "-p ${crate}( |$)"; then
    echo "ci.yml の package の検査に ${crate} が無い" >&2
    exit 1
  fi
done
# Phase 90（リリース判定）: サブパス配信（docs.yml と同じ --base-url）で、favicon の
# link が base の下を指し、そのファイルが dist にある（以前は link が無く、ブラウザが
# 探すドメイン直下の /favicon.ico は 404 だった）。docs/dist を作り直すので最後に置く
# ここで止まっても docs/dist を既定の base に戻す（/setoka/ 向けの成果物を残さない。終了コードは止まったときのまま。
# PR #114 のレビュー指摘）
trap 'rc=$?; if [ "$rc" -ne 0 ]; then if "$SETOKA" build --root docs -q >/dev/null 2>&1; then :; fi; fi; exit "$rc"' EXIT
"$SETOKA" build --root docs --base-url /setoka/
href="$(grep -oE '<link rel="icon" href="[^"]*"' docs/dist/index.html | sed -E 's/.*href="([^"]*)"/\1/')"
case "$href" in
  /setoka/*) ;;
  *) echo "favicon の href が base の下を指していない: '$href'" >&2; exit 1 ;;
esac
test -f "docs/dist/${href#/setoka/}"
grep -q '<footer class="site-footer">' docs/dist/index.html
grep -q 'class="header-links"' docs/dist/index.html
# 後で docs/dist を開く人のために、既定の base で作り直しておく
trap - EXIT
"$SETOKA" build --root docs -q
echo "docs のゲートはすべて通りました"
