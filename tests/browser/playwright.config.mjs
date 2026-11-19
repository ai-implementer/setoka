// setoka が出力するサイトのブラウザ側のスモークテスト（v0.22 Phase 100）。
// 2 つのサイトを `setoka preview` で配信して確かめる:
// - docs サイト（docs/。図は backend = "ssr" なので client 描画の図は無い）
// - `setoka new` の雛形から作ったサイト（既定の client 描画の Mermaid と KaTeX がある）
// どちらも先に `setoka build` しておく（手順は README.md）。
import { defineConfig, devices } from "@playwright/test";

const bin = process.env.SETOKA_BIN || "setoka";
const site = process.env.SETOKA_SITE;
if (!site) {
  throw new Error("SETOKA_SITE に、setoka new で作って build したサイトのディレクトリを指定してください");
}

// smoke.spec.mjs の DOCS・SITE と同じ値にする
const DOCS_URL = "http://127.0.0.1:5310";
const SITE_URL = "http://127.0.0.1:5311";

export default defineConfig({
  testDir: ".",
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: {
    locale: "ja-JP",
    // 配色の切替の順番（OS の設定 → 今の見た目の反対 → …）を決めるため、OS の設定はライトにそろえる
    colorScheme: "light",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: [
    {
      command: `"${bin}" preview --root ../../docs --port 5310`,
      url: `${DOCS_URL}/`,
      reuseExistingServer: !process.env.CI,
    },
    {
      command: `"${bin}" preview --root "${site}" --port 5311`,
      url: `${SITE_URL}/`,
      reuseExistingServer: !process.env.CI,
    },
  ],
});
