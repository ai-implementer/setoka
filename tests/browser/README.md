# ブラウザ側のスモークテスト

setoka が出力するサイトを実際のブラウザで開き、検索・配色の切替と保存・サイドバーの位置・`/` キー・
コピーボタン・client 描画の Mermaid と KaTeX が動くかを確かめる（v0.22 Phase 100）。CI では ci.yml の
`browser` ジョブが PR ごとに動かす。

- 道具は [Playwright](https://playwright.dev/)。版は `package.json` で完全一致に固定している（dependabot の
  npm が上げる）。setoka のビルドには Node も Playwright も使わない
- 確かめるサイトは 2 つ。docs サイト（`docs/`。図は `backend = "ssr"`）と、`setoka new` の雛形から作った
  サイト（既定の client 描画の図と数式がある）。どちらも `setoka preview` で配信する
  （docs はポート 5310、雛形のサイトは 5311）

## 手元で動かす

Node が要る（CI と開発コンテナは Node 26。開発コンテナには入っている）。リポジトリのルートで:

```bash
cargo build -p setoka-cli
S="$CARGO_TARGET_DIR/debug/setoka"   # 開発コンテナの外なら ./target/debug/setoka
"$S" build --root docs
"$S" new /tmp/setoka-browser-site && "$S" build --root /tmp/setoka-browser-site

cd tests/browser
npm install
npx playwright install --with-deps chromium   # 開発コンテナでは作り直すたびに要る（足りないライブラリを sudo で入れる。mac では --with-deps は不要）
SETOKA_BIN="$S" SETOKA_SITE=/tmp/setoka-browser-site npx playwright test
```

`SETOKA_BIN` が相対パスなら `tests/browser` から見たパスにする。失敗したテストの画面とトレースは
`test-results/` に残る（`npx playwright show-trace <trace.zip>` で開ける）。
