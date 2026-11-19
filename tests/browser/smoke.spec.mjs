// setoka が出力するサイトのブラウザ側のスモークテスト（v0.22 Phase 100）。
// 人手で確かめていた所（検索・配色の切替と保存・サイドバーの位置・キーボード・コピー・
// client 描画の図と数式）を、実際のブラウザで確かめる。
import { test, expect } from "@playwright/test";

// playwright.config.mjs の webServer と同じ値にする
const DOCS = "http://127.0.0.1:5310"; // docs サイト（図は SSR）
const SITE = "http://127.0.0.1:5311"; // 雛形から作ったサイト（図は client 描画）

// 捕まえられなかった JS の例外を集める（テストの最後に空であることを確かめる）
function collectPageErrors(page) {
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  return errors;
}

test.describe("docs サイト", () => {
  test("ヘッダーの検索欄で結果が出る", async ({ page }) => {
    const errors = collectPageErrors(page);
    await page.goto(`${DOCS}/`);
    await page.locator("#setoka-search-input").fill("設定");
    const results = page.locator("#setoka-search-results");
    await expect(results).toBeVisible();
    await expect(results.locator("a.search-hit").first()).toBeVisible();
    await expect(results.locator('a.search-hit[href*="reference/config"]').first()).toBeVisible();
    expect(errors).toEqual([]);
  });

  test("検索結果のページに結果が出る", async ({ page }) => {
    const errors = collectPageErrors(page);
    await page.goto(`${DOCS}/search/?q=${encodeURIComponent("設定")}`);
    const root = page.locator("#setoka-search-page");
    await expect(root.locator("a.search-hit").first()).toBeVisible();
    await expect(root.locator('a.search-hit[href*="reference/config"]').first()).toBeVisible();
    expect(errors).toEqual([]);
  });

  test("/ キーで検索欄に移る", async ({ page }) => {
    await page.goto(`${DOCS}/guide/`);
    // 読み込んだ直後は入力欄にフォーカスが無いので、そのまま / を押す
    await page.keyboard.press("/");
    await expect(page.locator("#setoka-search-input")).toBeFocused();
  });

  test("配色の切替は OS の設定 → ダーク → ライト → OS の設定 の順で、再読み込み後も残る", async ({ page }) => {
    const errors = collectPageErrors(page);
    await page.goto(`${DOCS}/`);
    const html = page.locator("html");
    const toggle = page.locator("#theme-toggle");
    const saved = () => page.evaluate(() => localStorage.getItem("setoka-theme"));

    // 選んでいないあいだは data-theme を付けない（OS の設定に従う）
    await expect(html).not.toHaveAttribute("data-theme");
    // OS の設定はライトなので、最初の 1 回は反対のダーク
    await toggle.click();
    await expect(html).toHaveAttribute("data-theme", "dark");
    expect(await saved()).toBe("dark");

    // 再読み込みしても選んだ配色が残る（head 内の script が最初のペイントの前に付ける）
    await page.reload();
    await expect(html).toHaveAttribute("data-theme", "dark");

    await toggle.click();
    await expect(html).toHaveAttribute("data-theme", "light");
    expect(await saved()).toBe("light");

    // OS の設定に戻すと、data-theme と保存した選択を消す
    await toggle.click();
    await expect(html).not.toHaveAttribute("data-theme");
    expect(await saved()).toBeNull();
    expect(errors).toEqual([]);
  });

  test("サイドバーのリンクで移っても、サイドバーの位置が残る", async ({ page }) => {
    // サイドバーが独立してスクロールするよう、画面の高さを低くする
    await page.setViewportSize({ width: 1280, height: 480 });
    await page.goto(`${DOCS}/guide/install/`);
    const sidebar = page.locator("#site-sidebar");
    const scrollable = await sidebar.evaluate((el) => el.scrollHeight > el.clientHeight);
    expect(scrollable, "サイドバーがスクロールしない（画面の高さを見直す）").toBe(true);

    // 少しスクロールして、見えている範囲の中の別のページへのリンクを選ぶ
    const target = await sidebar.evaluate((el) => {
      el.scrollTop = Math.min(160, el.scrollHeight - el.clientHeight);
      const top = el.scrollTop;
      const links = Array.from(el.querySelectorAll("a[href]"));
      const visible = links.find((a) => {
        if (a.closest("li.active")) return false;
        const y = a.offsetTop;
        return a.offsetParent === el && y > top + 40 && y + a.offsetHeight < top + el.clientHeight - 40;
      });
      return { top, href: visible ? visible.getAttribute("href") : null };
    });
    expect(target.href, "見えている範囲にリンクが無い").not.toBeNull();
    // 位置の保存はスクロールの 200 ms 後なので、それを待ってから移る
    await page.waitForTimeout(400);
    await Promise.all([page.waitForURL((url) => url.href !== `${DOCS}/guide/install/`), sidebar.locator(`a[href="${target.href}"]`).first().click()]);

    const after = await page.locator("#site-sidebar").evaluate((el) => el.scrollTop);
    expect(Math.abs(after - target.top)).toBeLessThanOrEqual(2);
  });
});

test.describe("雛形から作ったサイト", () => {
  test("client 描画の Mermaid の図がすべて SVG になる", async ({ page }) => {
    const errors = collectPageErrors(page);
    await page.goto(`${SITE}/guide/examples/`);
    const blocks = page.locator("pre.mermaid");
    const count = await blocks.count();
    expect(count).toBeGreaterThan(0);
    await expect(page.locator("pre.mermaid svg")).toHaveCount(count, { timeout: 30_000 });
    // 書き間違いがあると mermaid はエラーの図（"Syntax error"）を描く
    const broken = await blocks.evaluateAll((els) => els.filter((el) => el.textContent.includes("Syntax error")).length);
    expect(broken).toBe(0);
    expect(errors).toEqual([]);
  });

  test("数式が KaTeX で描かれる", async ({ page }) => {
    const errors = collectPageErrors(page);
    await page.goto(`${SITE}/guide/examples/`);
    // katex-init.js は [data-math-style] の要素を .math（中に .katex）に置き換える。
    // 雛形の「記法の例」の数式は、インライン・$$ のブロック・```math の 3 つ
    await expect(page.locator(".markdown-body .math .katex")).toHaveCount(3);
    await expect(page.locator(".markdown-body [data-math-style]")).toHaveCount(0);
    await expect(page.locator(".markdown-body .katex-error")).toHaveCount(0);
    expect(errors).toEqual([]);
  });

  test("コードブロックのコピーボタンで中身をコピーできる", async ({ page, context }) => {
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);
    await page.goto(`${SITE}/guide/examples/`);
    const pre = page.locator(".markdown-body pre:has(> code)").filter({ has: page.locator(".copy-button") }).first();
    const button = pre.locator(".copy-button");
    await expect(button).toHaveText("コピー");
    await button.click();
    await expect(button).toHaveText("コピーしました");
    // copy-button.js は code の textContent をそのままコピーする
    const copied = await page.evaluate(() => navigator.clipboard.readText());
    const code = await pre.locator("code").evaluate((el) => el.textContent);
    expect(copied).toBe(code);
  });
});
