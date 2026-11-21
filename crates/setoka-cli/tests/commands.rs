//! 配信しないコマンドの結合テスト（v0.22 Phase 101。10-12 レビューの R10）。
//! 終了コードの規約（0 = 成功 / 1 = 違反あり / 2 = 実行エラー）と出力の要点を、
//! ビルドした setoka で確かめる

mod common;

use common::Project;

#[test]
fn check_は問題が無ければ_0_リンク切れで_1_設定の誤りで_2() {
    let project = Project::new();
    let ok = project.run(&["check"]).expect(0);
    assert!(ok.stdout.contains("問題ありません"), "{}", ok.stdout);

    project.write("content/index.md", "# トップ\n\n[壊れたリンク](nai.md)\n");
    let broken = project.run(&["check"]).expect(1);
    assert!(
        broken.stdout.contains("error[broken-link]") && broken.stdout.contains("nai.md"),
        "{}",
        broken.stdout
    );

    project.write("setoka.toml", "[site]\ntitle = \"テスト\"\nnai = 1\n");
    let config = project.run(&["check"]).expect(2);
    assert!(
        config.stderr.contains("setoka.toml:3:1") && config.stderr.contains("site.nai"),
        "{}",
        config.stderr
    );
}

#[test]
fn lint_は表記ゆれで_1_fix_で直し_2_回目は書き込まない() {
    let project = Project::new();
    project.write("content/index.md", "# トップ\n\nＲｕｓｔで書く。\n");
    let found = project.run(&["lint"]).expect(1);
    assert!(
        found.stdout.contains("warning[fullwidth-alphanumeric]"),
        "{}",
        found.stdout
    );

    let fixed = project.run(&["lint", "--fix"]).expect(0);
    assert!(
        fixed.stdout.contains("1 件を自動修正しました"),
        "{}",
        fixed.stdout
    );
    assert_eq!(
        project.read("content/index.md"),
        "# トップ\n\nRustで書く。\n"
    );

    // 直すものが無ければ書き込まない（mtime を汚さない）
    let mtime = || {
        std::fs::metadata(project.path("content/index.md"))
            .unwrap()
            .modified()
            .unwrap()
    };
    let before = mtime();
    let again = project.run(&["lint", "--fix"]).expect(0);
    assert!(!again.stdout.contains("修正:"), "{}", again.stdout);
    assert_eq!(mtime(), before);
}

#[test]
fn search_はビルドした索引から探し_索引が無ければ_2() {
    let project = Project::new();
    project.write(
        "content/index.md",
        "# トップ\n\nライブリロードの説明です。\n",
    );
    let missing = project.run(&["search", "ライブリロード"]).expect(2);
    assert!(
        missing.stderr.contains("setoka build"),
        "{}",
        missing.stderr
    );

    project.run(&["build"]).expect(0);
    let hit = project.run(&["search", "ライブリロード"]).expect(0);
    assert!(
        hit.stdout.contains("全 1 件") && hit.stdout.contains("トップ"),
        "{}",
        hit.stdout
    );
    let none = project.run(&["search", "存在しない語句"]).expect(0);
    assert!(
        none.stdout.contains("一致するページはありませんでした"),
        "{}",
        none.stdout
    );
}

#[test]
fn llms_は_dist_無しで_llms_txt_を標準出力に出す() {
    let project = Project::new();
    let index = project.run(&["llms"]).expect(0);
    assert!(
        index.stdout.starts_with("# テスト\n") && index.stdout.contains("- [トップ]("),
        "{}",
        index.stdout
    );
    let full = project.run(&["llms", "--full"]).expect(0);
    assert!(full.stdout.contains("版その一"), "{}", full.stdout);
    assert!(!project.exists("dist"));
}

#[test]
fn build_drafts_は_draft_のページも出し_外して作り直すと消える() {
    let project = Project::new();
    project.write(
        "content/wip.md",
        "---\ndraft: true\n---\n\n# 下書き\n\n書きかけ。\n",
    );
    project.run(&["build"]).expect(0);
    assert!(!project.exists("dist/wip/index.html"));

    project.run(&["build", "--drafts"]).expect(0);
    assert!(project.read("dist/wip/index.html").contains("書きかけ。"));

    // 前回の出力に残った draft のページは、次の通常のビルドが掃除する
    project.run(&["build"]).expect(0);
    assert!(!project.exists("dist/wip/index.html"));
    assert!(project.exists("dist/index.html"));
}
