//! 配信と監視の結合テスト（v0.22 Phase 101。10-07 見直しの E3）。
//! `build --watch`・`dev` は原稿を編集すると作り直すこと、`dev`・`preview` が配信する
//! ページを、ビルドした setoka と HTTP で確かめる（ci.yml の e2e は「何もしなければ
//! 作り直さない」側だけを見ている）。
//!
//! TCP で待ち受けるので、ネットワークを禁じたサンドボックスの中では失敗する

// 結合テストは書き込みの口の縛り（ワークスペースの clippy.toml。v0.22 Phase 106）の対象外
#![allow(clippy::disallowed_methods)]

mod common;

use common::Project;

/// 原稿を編集して、`dist/index.html` が新しい中身になるまで待つ
fn edit_and_wait(project: &Project, server: &common::Server) {
    assert!(project.read("dist/index.html").contains("版その一"));
    project.write("content/index.md", "# トップ\n\n版その二\n");
    server.wait_until("dist/index.html が作り直されません", || {
        project.read("dist/index.html").contains("版その二")
    });
}

#[test]
fn build_watch_は原稿を編集すると作り直す() {
    let project = Project::new();
    let server = project.serve(&["build", "--watch"]);
    edit_and_wait(&project, &server);
}

#[test]
fn dev_は編集した原稿を配信し直し_ページにライブリロードを入れる() {
    let project = Project::new();
    let server = project.serve(&["dev"]);
    let page = server.get("/").unwrap();
    assert_eq!(page.status, 200);
    assert!(page.body.contains("版その一"), "{}", page.body);
    assert!(page.body.contains("js/livereload.js"), "{}", page.body);

    edit_and_wait(&project, &server);
    server.wait_until("配信するページが新しくなりません", || {
        server
            .get("/")
            .is_some_and(|page| page.status == 200 && page.body.contains("版その二"))
    });
}

#[test]
fn preview_は_dist_を配信し_無いページは_404() {
    let project = Project::new();
    project.run(&["build"]).expect(0);
    let server = project.serve(&["preview"]);

    let page = server.get("/").unwrap();
    assert_eq!(page.status, 200);
    assert!(page.body.contains("版その一"), "{}", page.body);
    assert!(
        page.header("content-type")
            .is_some_and(|v| v.starts_with("text/html")),
        "{}",
        page.head
    );
    // preview は配信だけで、ライブリロードもオートリフレッシュも入れない
    assert!(!page.body.contains("js/livereload.js"));
    assert!(!page.body.contains("js/autorefresh.js"));

    let missing = server.get("/nai/").unwrap();
    assert_eq!(missing.status, 404);
}
