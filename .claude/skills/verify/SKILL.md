---
name: verify
description: setoka の変更を検証する。CI 相当（fmt / clippy / test / package / wasm check / docs サイト検証）＋ CLI 実機 e2e を、既知の罠を回避した正しい順序で実行する。コード変更後の検証・コミット前チェックで使う。
---

# setoka 検証手順

CI（.github/workflows/ci.yml）と同等＋実機 e2e。上から順に実行する。

## 1. 静的チェック

```bash
cargo fmt --all --check
cargo machete   # 未使用依存の検出（要 cargo install cargo-machete。CI にもある）
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo deny check bans licenses sources   # 要 cargo-deny（版は下記）。CI の deny ジョブと同じ
```

- clippy はルートの `clippy.toml` で std::fs の書き込み・削除・作成（`fs::write`・`remove_*`・`rename`・`copy`・
  `create_dir(_all)`・`File::create(_new)`・`OpenOptions::open`）を禁じている（10-12 レビューの R8。v0.22 Phase 106）。
  落ちたら出力先は `setoka_core::output` の口（実体は kunenbo）、原稿は `commands::rewrite_source` を通す。
  テストのコードは各 crate のルートの `#![cfg_attr(test, allow(clippy::disallowed_methods))]` と結合テストのファイルの
  先頭の `#![allow(…)]` で許している（結合テストのファイルを足したら先頭に書く）。mikan は当面 crate の `clippy.toml` で
  縛りを外している（ソースが検索 wasm の入力のハッシュに入るため。次に wasm を作り直すときに消す）。既にある clippy.toml を書き換えたときは clippy が
  気付いて検査し直すが、**新しく clippy.toml を置いたとき**（mikan のように crate に置くなど）は気付かず前の結果を
  再利用するので、`touch crates/*/src/lib.rs crates/setoka-cli/src/main.rs` してから実行する。一覧は std::fs に加えて
  tokio::fs・リンクの作成・`set_permissions`・`kunenbo::write_if_changed` も禁じている（`clippy.toml` のコメント）
- machete の false positive は該当 crate の Cargo.toml に
  `[package.metadata.cargo-machete] ignored = ["<crate>"]` を書いて抑制する。
- CI の cargo は `--locked` 付き（Phase 82）。Cargo.lock の更新が要る変更は、手元で lock を
  更新してコミットに含める（CI では lock を書き換えられず失敗する）。
- cargo-deny は CI の `EmbarkStudios/cargo-deny-action` が使う版に合わせる
  （v2.1.2 → cargo-deny 0.21.0。`cargo install cargo-deny --version 0.21.0 --locked`）。
  設定は `deny.toml`。依存（Cargo.toml・Cargo.lock）を触ったら `cargo deny check advisories` と
  `scripts/vendor-advisories.sh` も流す（CI では deps.yml が PR で動く）。
  失敗したときの扱いは `dependency-update` スキル。

## 2. テスト

```bash
cargo test --locked --workspace --exclude jabara --exclude setoka-cli
cargo test --locked -p setoka-cli --test commands --test docs_excerpt
# ↓ サンドボックス外で実行する（TCP で待ち受ける）
cargo test --locked -p jabara
cargo test --locked -p setoka-cli --bins --test serve
```

- **TCP で待ち受けるテストはサンドボックス外必須**: jabara（旧 setoka-server）の serve テスト・setoka-cli の単体テスト（`--bins`。`commands/extlink.rs` の外部リンク検査の 3 本がローカルにサーバを立てる）・setoka-cli の結合テスト `crates/setoka-cli/tests/serve.rs`（`build --watch`・`dev`・`preview` を起動して HTTP で取る。v0.22 Phase 101）は、サンドボックス内では PermissionDenied で落ちる（コード起因ではない）。CI はまとめて `cargo test --workspace` で動かす。
- **insta スナップショット差分が出たら**: 差分が意図どおりか必ず目視 → `INSTA_UPDATE=always cargo test -p <crate>` で更新 → `git diff` で更新内容を再確認。意図しない差分は変更側のバグを疑う。CI は `INSTA_UPDATE=no` で未承認を失敗にする。

## 3. ビルドと crates.io パッケージ検証

```bash
cargo build --locked --workspace
cargo package --locked -p tankan -p mikan -p kabosu -p shikuwasa -p amanatsu -p hebesu -p jabara -p kunenbo
scripts/third-party-licenses.sh check
scripts/build-search-wasm.sh --check   # 検索 wasm の作り直し忘れ（Phase 89。ビルドのツールは要らない）
git grep -n -i 'y[u]zu' -- . ':!ROADMAP.md' ':!ROADMAP-history.md'   # 旧名の残り（v0.21 Phase 94。何も出なければよい）
scripts/check-doc-refs.sh              # 文書とコメントが指すパス・スキル・ROADMAP / CLAUDE.md の節が実在するか（Phase 93）
```

- `cargo package` は公開対象 8 crate のメタデータ・同梱内容の回帰を検出する（CI にもある）。
  CI は加えて `cargo package --list` で 8 crate とも `LICENSE-MIT` / `LICENSE-APACHE` を含み、
  tankan に `tests/corpus` が入らないことを見る。
- `third-party-licenses.sh check` は two-face の一覧の版（Phase 81）に加えて、使ってよい
  ライセンスの一覧が `licenses/about.toml` と `deny.toml` で同じか、検索 wasm の通知に載った
  crate と版の集合が mikan-wasm の wasm32 向け依存（`cargo tree`）と一致するか
  （= wasm が今の依存で作られているか）を見る（Phase 82）。
  kabosu と hebesu は加えて package 後 manifest の依存ゼロ検査（CI）、kabosu は
  `cargo check -p kabosu --target thumbv7em-none-eabi`（no_std 担保）がある。
  **作業ツリーが dirty だと拒否される**ので、コミット後に走らせるか意図を確認して `--allow-dirty`。

## 4. wasm32 チェック

```bash
cargo check --locked -p mikan-wasm --target wasm32-unknown-unknown
cargo check --locked -p tankan --target wasm32-unknown-unknown
```

## 5. docs サイト検証（このリポジトリ自身のドキュメントサイト）

**`docs/` の原稿・テーマ・記法まわりを触ったら必須。** CI（ci.yml の docs ステップ）と同じスクリプトを呼ぶ
（v0.22 Phase 107。以前はここに CI のゲートの一部を手で写していた）。

```bash
cargo build -p setoka-cli
scripts/docs-gates.sh          # バイナリは ${CARGO_TARGET_DIR:-target}/debug/setoka。別のものは引数で渡す
cmp crates/setoka-cli/scaffold/deploy.yml docs/snippets/deploy.yml  # docs の写しは雛形とバイト一致（CI の scaffold actions pins。pr-check.yml の SHA とインストール手順も照合する）
```

- `scripts/docs-gates.sh` は `setoka check --root docs`・`build --root docs`・機能ごとの配信ゲート（生成物の grep）・
  検索の照合（`setoka search`。jq が要る）・SSR フォールバックの検出・`--base-url /setoka/` の favicon の確認・
  公開する crate が文書（docs の「開発者向け」・CLAUDE.md・`publish-crate` スキル・ci.yml の package の検査）に
  載っているかの照合を行う。止まったら、止まった行と文言が出る
- **新しい機能・原稿の節のゲートはスクリプトに足す**（ci.yml とこのスキルには書かない）。ゲートを素通りさせる書き方
  （`&&`・`||`・`!` で始まる文・`set +e`。`bash -e` は `A && B` の A の失敗などで止まらない）は使えず、スクリプトが起動時に自分を
  調べて止まる
- `docs/setoka.toml` の `[markdown]` とその下の表（`[markdown.…]`）は、`docs/content/guide/code-and-math.md`
  の `lines=` で引用されている（書き方の例も同じ範囲）。引用と例の範囲がそろっているか・その範囲がこれらの表を漏れなく・余分なく
  指しているか・docs をビルドしたときのキャプション（`setoka.toml:行範囲`）がその範囲かは、setoka-cli の結合テスト
  （`crates/setoka-cli/tests/docs_excerpt.rs`。2 節の `--test docs_excerpt`。docs を一時ディレクトリへ写してビルドする）が確かめる。
  `docs/setoka.toml` の行を動かしたら、テストのメッセージの案内に従って表の並びか `lines=`（引用と例。説明の文の例も合わせる）を
  直す。スクリプトは原稿も TOML も読まない
- 終わったときの `docs/dist/` は既定の base で作り直してある（`run` スキルで開ける）

## 6. e2e（CLI 実機）

**必ず先に `cargo build -p setoka-cli` を実行する。** `cargo test` は setoka-cli の結合テストのために
`target/debug/setoka` を作り直すが、`-p` でほかの crate だけを試したときは作り直さない（古いバイナリで確かめてしまう）。

```bash
cargo build -p setoka-cli
./target/debug/setoka new "<scratchpad>/e2e-docs"
# --root（Phase 72）は cwd がプロジェクト外の状態で見る = cd より前に置く。
# 前後どちらでも効く / 上方向探索をしない / ルート自身がリンクでも通る / new は拒否
./target/debug/setoka --root "<scratchpad>/e2e-docs" build
./target/debug/setoka build --root "<scratchpad>/e2e-docs"
mkdir -p "<scratchpad>/root-empty"
./target/debug/setoka check --root "<scratchpad>/root-empty"   # exit 2・文言に「上方向」を含まない
mkdir -p "<scratchpad>/e2e-docs/sub" && ./target/debug/setoka check --root "<scratchpad>/e2e-docs/sub"  # exit 2
ln -s "<scratchpad>/e2e-docs" "<scratchpad>/root-link" && ./target/debug/setoka build --root "<scratchpad>/root-link"
./target/debug/setoka new --root "<scratchpad>/e2e-docs" "<scratchpad>/root-new"   # exit 2
cd "<scratchpad>/e2e-docs"
test -f .github/workflows/deploy.yml   # Pages デプロイ雛形の同梱
# 雛形 deploy.yml は setoka new した版のリリースを SHA256SUMS で照合して使う（Phase 77・84）
v=$(<repo>/target/debug/setoka --version | awk '{print $2}')
grep -qF "SETOKA_VERSION: $v" .github/workflows/deploy.yml && grep -qF '| sha256sum -c -' .github/workflows/deploy.yml && grep -q -- "--tag v$v setoka-cli" .github/workflows/deploy.yml && echo "OK deploy version"
<repo>/target/debug/setoka build
test -f dist/index.html && test -f dist/_search/manifest.json && test -f dist/_search/search_bg.wasm
# 雛形のナビ: ガイドの後ろに用語集・ホームの次のページはガイド（Phase 84）
grep -q '<a class="pager-next" rel="next" href="/guide/">' dist/index.html && echo "OK pager"
grep -oE 'href="/(guide|glossary)/"' dist/index.html | head -2   # href="/guide/" → href="/glossary/" の順
# 第三者ライセンスの通知を資産の隣に配り、vendor の開発用メモは配らない（Phase 81）
test -s dist/_assets/vendor/THIRD-PARTY-LICENSES.txt && test -s dist/_search/THIRD-PARTY-LICENSES.txt
test ! -e dist/_assets/vendor/README.md
# 原稿の .md の配信（Phase 83）: 既定は配信、llms.page_md = false で全ページ止める
# （残っていた .md は孤児掃除で消え、data-md-url も消え、llms.txt は HTML を指す）。
# frontmatter の pageMd: false はそのページだけ。リダイレクトは絶対パスで書く（相対はフックが止める）
test -f dist/index.md && grep -q 'data-md-url' dist/index.html
printf '\n[llms]\npage_md = false\n' >> "<scratchpad>/e2e-docs/setoka.toml" && <repo>/target/debug/setoka build
find dist -name '*.md' | grep -q . && echo "NG: .md が残った"
grep -q 'data-md-url' dist/index.html && echo "NG: data-md-url"
grep -q '(/guide/getting-started/)' dist/llms.txt && echo "OK llms は HTML"
sed -i 's/^page_md = false$/page_md = true/' setoka.toml
sed -i '0,/^order: 1$/s//order: 1\npageMd: false/' content/guide/getting-started.md && <repo>/target/debug/setoka build
test -f dist/index.md && test ! -e dist/guide/getting-started.md && echo "OK pageMd"
sed -i '/^pageMd: false$/d' content/guide/getting-started.md && <repo>/target/debug/setoka build
# output.clean = false で止めてから --force（出力マニフェストも消える）しても .md を残さない（PR #42 の指摘）
sed -i 's/^clean = true$/clean = false/' setoka.toml && <repo>/target/debug/setoka build && test -f dist/index.md
sed -i 's/^page_md = true$/page_md = false/' setoka.toml && <repo>/target/debug/setoka build --force
find dist -name '*.md' | grep -q . && echo "NG: --force 後に .md が残った"
sed -i 's/^page_md = false$/page_md = true/; s/^clean = false$/clean = true/' setoka.toml && <repo>/target/debug/setoka build
<repo>/target/debug/setoka search "はじめに" | grep "はじめに"
# タイポトレランス（出力の有無だけでなくヒット内容まで見る）とフレーズ検索の正/逆順
<repo>/target/debug/setoka search "ダーくモード" | grep -q "ダークモード"
<repo>/target/debug/setoka search '"ライブリロード"' | grep -q "ライブリロード"
<repo>/target/debug/setoka search '"リロードライブ"' | grep -q "一致するページはありませんでした"
# -q / -v / search --format（Phase 73）: -q は RUST_LOG より優先して info 以下を黙らせる
RUST_LOG=debug <repo>/target/debug/setoka build --force -q 2>&1 | wc -l        # 0
# grep -q は最初の一致で読み手を閉じる = stderr の EPIPE でビルドが落ちない検査も兼ねる
<repo>/target/debug/setoka build --force -v 2>&1 | grep -q DEBUG && echo "OK verbose"
test "${PIPESTATUS[0]}" -eq 0 && test -f dist/_search/manifest.json && echo "OK 完走"
<repo>/target/debug/setoka build -q -v                                          # exit 2（同時指定は矛盾）
<repo>/target/debug/setoka -q build -v                                          # exit 2（前後に分けても弾く）
<repo>/target/debug/setoka search --format json "はじめに" | head -1 | grep -q '^\[$' && echo "OK search json"
<repo>/target/debug/setoka search --json "はじめに" | head -1 | grep -q '^\[$' && echo "OK 旧表記"
<repo>/target/debug/setoka search --json --format json "はじめに"              # exit 2（旧表記との同時指定）
# シェル補完（Phase 74）: 全シェルで生成でき、グローバル引数が候補に入る。bash は構文検査まで
for sh in bash zsh fish powershell elvish; do <repo>/target/debug/setoka completions $sh | grep -q quiet || echo "NG $sh"; done
<repo>/target/debug/setoka completions bash > "<scratchpad>/setoka.bash" && bash -n "<scratchpad>/setoka.bash" && echo "OK bash -n"
<repo>/target/debug/setoka completions bash --root .                            # exit 2（プロジェクトを読まない）
# build の報告とログ（Phase 85。CI の「e2e (build の報告とログ)」と同じ。別のプロジェクトで見る）:
# リンク切れは 2 回目（キャッシュに当たる）の build でも 1 行出て exit 0・パイプ先に色と時刻が出ない・
# キャッシュの内訳は -v だけ・Guide.md と guide/ は route-case-conflict・check --no-fmt・
# fmt がパーミッションを引き継ぐ・原稿 0 件の警告。手順は ci.yml の該当ステップをそのまま流す
# --base-url は設定より優先（deploy.yml が configure-pages の base_path を渡す契約）
<repo>/target/debug/setoka build --base-url /docs/ && grep -q '/docs/_assets/' dist/index.html
# 共有カード（Phase 76）: パスだけの base では og:image が出ず、フル URL なら canonical / og:url / og:image が絶対 URL で出る
sed -i 's|^title = "My Docs"$|title = "My Docs"\nimage = "/images/setoka-logo.svg"|' setoka.toml
<repo>/target/debug/setoka build --base-url /docs/ && grep -q 'og:image' dist/index.html && echo "NG: パス base で og:image"
<repo>/target/debug/setoka build --base-url https://example.com/docs/
grep -q 'rel="canonical" href="https://example.com/docs/guide/getting-started/"' dist/guide/getting-started/index.html && echo "OK canonical"
grep -q 'og:image" content="https://example.com/docs/images/setoka-logo.svg"' dist/index.html && echo "OK og:image"
<repo>/target/debug/setoka build   # 後続の検査は既定 base_url に戻してから
# 監視（Phase 77）: 何も編集せず build --watch を数秒動かして再ビルド 0 回（Linux の inotify は
# 開いただけのイベントも届ける。macOS では起きないので開発コンテナで見る）
timeout 6 <repo>/target/debug/setoka build --watch --port 5199 2>&1 | grep -c '変更を検知'   # 0
# frontmatter の閉じ忘れは check で error（exit 1）・区切り線で始まる文書は誤検出しない（Phase 77）
printf -- '---\ntitle: 閉じ忘れ\n\n本文\n' > content/unclosed.md
printf -- '---\n\n# 区切り線で始まる\n\n本文\n' > content/hr-first.md
<repo>/target/debug/setoka check --format json 2>/dev/null | grep -B2 '"path": "content/' | grep -c frontmatter-unrecognized   # 1（unclosed.md だけ）
rm content/unclosed.md content/hr-first.md
# frontmatter の書き間違いは全ページ分を日本語・ファイル上の行と列で出し exit 2（Phase 88。CI と同じ）
printf -- '---\n\ntitle: a\norder: "最初"\ndraft: yes\n---\n\n# a\n' > content/fm-a.md
printf -- '---\naliases: old/\n---\n\n# b\n' > content/fm-b.md
<repo>/target/debug/setoka build 2>&1 | grep -E '（3 件）|fm-a.md:4:8|fm-a.md:5:8|fm-b.md:2:10' | wc -l   # 4・exit 2
rm content/fm-a.md content/fm-b.md
<repo>/target/debug/setoka fmt --check && <repo>/target/debug/setoka lint && <repo>/target/debug/setoka check
# 異常系: 壊れリンクを注入して check が終了コード 1 を返すこと（CI と同じ）
echo '[壊れリンク](missing.md)' >> content/index.md
<repo>/target/debug/setoka check && echo "NG: 検出漏れ" || echo "OK"

# 機械可読出力（診断が出ている状態のまま検証する）
# json は単一オブジェクトで、標準出力に他の行を混ぜない
<repo>/target/debug/setoka check --format json | head -1 | grep -q '^{$' && echo "OK json"
# github は注釈行を出す。GITHUB_WORKSPACE を差し替えると相対パスが付け替わる
# （これが崩れると PR に注釈が紐づかない。--root 指定で cwd がプロジェクト外でも同じ =
#   docs.yml / docs-links.yml がリポジトリルートから --root docs で実行する前提）
<repo>/target/debug/setoka check --format github | grep '^::error file='
GITHUB_WORKSPACE="$(dirname "$PWD")" <repo>/target/debug/setoka check --format github | grep '^::error file=e2e-docs/'
# cwd をプロジェクト外へ出してから --root（相対）で指定する（サブシェルなので後続に影響しない）
( cd .. && GITHUB_WORKSPACE="$PWD" <repo>/target/debug/setoka check --root e2e-docs --format github | grep '^::error file=e2e-docs/' )
# lint --fix と併用しても標準出力は JSON のまま（進捗は stderr へ逃げる）
<repo>/target/debug/setoka lint --fix --format json 2>/dev/null | head -1 | grep -q '^{$' && echo "OK fix+json"
# preview のリンク遮断（Phase 65）: dist にリンクを置いて 404 と内容非漏洩を見る
mkdir -p /tmp/outside && echo '<html>secret</html>' > /tmp/outside/secret.html
ln -s /tmp/outside dist/link && ln -s /tmp/outside/secret.html dist/leaf.html
<repo>/target/debug/setoka preview --port 48124 >/dev/null 2>&1 & GUARD_PID=$!
sleep 1
test "$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:48124/link/secret.html)" = 404 && echo "OK link 404"
! curl -s http://127.0.0.1:48124/leaf.html | grep -q secret && echo "OK 非漏洩"
# Host の検査（Phase 83。DNS リバインディング対策）: ホスト名は 403、localhost は 200
test "$(curl -s -o /dev/null -w '%{http_code}' -H 'Host: evil.example' http://127.0.0.1:48124/)" = 403 && echo "OK host 403"
test "$(curl -s -o /dev/null -w '%{http_code}' -H 'Host: localhost:48124' http://127.0.0.1:48124/)" = 200 && echo "OK localhost"
kill $GUARD_PID; rm dist/link dist/leaf.html
# 外部リンク検査（opt-in・Phase 66）: ネットワークへは出ず、自分の preview を相手にする
<repo>/target/debug/setoka preview --port 48123 >/dev/null 2>&1 & PREVIEW_PID=$!
sleep 1
printf '\n[ok](http://127.0.0.1:48123/)\n\n[missing](http://127.0.0.1:48123/no-such-file)\n\n[down](http://127.0.0.1:1/)\n' >> content/index.md
<repo>/target/debug/setoka check --external-links --format json > /tmp/extlink.json; kill $PREVIEW_PID
grep -q '"rule": "external-link-broken"' /tmp/extlink.json && grep -q '"skipped": 1' /tmp/extlink.json && echo "OK extlink"
<repo>/target/debug/setoka check --format json | grep -q '"skipped": 0' && echo "OK 既定は触れない"
```

終了コード規約: 0 = 成功 / 1 = 違反あり / 2 = 実行エラー。

検索・OpenAPI・アセット周りを触ったときは CI の e2e にある以下も再現する:
**OpenAPI の `file:` 参照が仕様ファイルの変更だけで再ビルドに反映されること**、
**content 同伴アセット（ページ横の画像）が dist へコピーされ `src` が絶対 URL へ解決されること**。

## 7. UI・テーマ・scaffold の変更がある場合

`run` スキル（プロジェクト版）でブラウザ配信まで実機確認する。

## 8. ブラウザ側のスモークテスト（Node がある環境だけ）

CI の `browser` ジョブ（PR ごと）と同じテスト。検索・配色の切替と保存・サイドバーの位置・`/` キー・
コピーボタン・client 描画の Mermaid と KaTeX を Playwright で確かめる（v0.22 Phase 100）。
手順は `tests/browser/README.md`。開発コンテナには Node（26）が入っている（Chromium は入っていないので、
コンテナを作り直すたびに `npx playwright install --with-deps chromium` で入れる）。Node が無い環境では CI の結果を見る。
