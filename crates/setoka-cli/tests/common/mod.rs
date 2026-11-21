//! setoka-cli の結合テストの共通部分（v0.22 Phase 101）。
//!
//! ビルドした `setoka` を子プロセスで動かし、終了コード・出力・dist・配信した HTTP を見る。
//! `cargo test` に入るので、CI の test-cross（Windows・macOS）でも同じテストが動く。
//! HTTP は std の `TcpStream` で取る（deny.toml が HTTP クライアントの crate を禁止している）

// テストのファイルごとに別の crate になり、使う関数がファイルで違う
#![allow(dead_code)]

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// 待つ上限（CI の macOS・Windows の遅さを見込む）
pub const DEADLINE: Duration = Duration::from_secs(60);

/// 目印のページ（`content/marker.md`）の URL
const MARKER_PATH: &str = "/marker/";

/// 一時ディレクトリに作ったプロジェクト（`setoka.toml`・`content/index.md`・
/// プロジェクトごとに違う目印を書いた `content/marker.md`）
pub struct Project {
    root: tempfile::TempDir,
    /// 子プロセスのログの置き場。監視の対象（プロジェクトルート）の外に置く
    /// （中に置くと、ログを書くたびに変更として検知されて再ビルドが続く）
    logs: tempfile::TempDir,
    /// 配信を始めたのが自分の子プロセスかを見分ける文字列（[`Project::serve`]）
    marker: String,
}

impl Project {
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let project = Self {
            root: tempfile::tempdir().unwrap(),
            logs: tempfile::tempdir().unwrap(),
            marker: format!(
                "marker-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ),
        };
        project.write("setoka.toml", "[site]\ntitle = \"テスト\"\n");
        project.write("content/index.md", "# トップ\n\n版その一\n");
        project.write(
            "content/marker.md",
            &format!("# 目印\n\n{}\n", project.marker),
        );
        project
    }

    pub fn root(&self) -> &Path {
        self.root.path()
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root().join(rel)
    }

    pub fn write(&self, rel: &str, text: &str) {
        let path = self.path(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// 読めなければ空文字（再ビルドの途中でファイルが無い瞬間がある）
    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.path(rel)).unwrap_or_default()
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.path(rel).exists()
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_setoka"));
        cmd.arg("--root")
            .arg(self.root())
            .args(args)
            .env_remove("RUST_LOG")
            .stdin(Stdio::null());
        cmd
    }

    /// 終わるまで待って結果を返す
    pub fn run(&self, args: &[&str]) -> Run {
        let output = self.command(args).output().unwrap();
        Run {
            args: args.join(" "),
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// 配信するコマンド（`dev`・`preview`・`build --watch`）を `--port` を付けて起動し、
    /// 目印のページがこのプロジェクトの目印つきで返るまで待つ。ポートが使用中なら
    /// 別のポートでやり直す
    pub fn serve(&self, args: &[&str]) -> Server {
        for _ in 0..3 {
            let port = free_port();
            let log = self.logs.path().join(format!("{port}.log"));
            let file = fs::File::create(&log).unwrap();
            let child = self
                .command(args)
                .arg("--port")
                .arg(port.to_string())
                .stdout(file.try_clone().unwrap())
                .stderr(file)
                .spawn()
                .unwrap();
            let mut server = Server { child, log, port };
            match server.wait_ready(&self.marker) {
                Ready::Yes => return server,
                // 空きを調べてから起動するまでの間に、ほかのテストが同じポートを取った
                Ready::PortInUse => continue,
            }
        }
        panic!("空いているポートで起動できません");
    }
}

/// 終わったコマンドの結果
pub struct Run {
    args: String,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    /// 終了コードを確かめる（違えば出力ごと失敗にする）
    pub fn expect(self, code: i32) -> Self {
        assert_eq!(
            self.code,
            Some(code),
            "`setoka {}` の終了コード\n--- stdout\n{}\n--- stderr\n{}",
            self.args,
            self.stdout,
            self.stderr
        );
        self
    }
}

enum Ready {
    Yes,
    PortInUse,
}

/// 配信中の setoka。drop で止める（一時ディレクトリより先に止まるよう、テストでは
/// `Project` より後に作る。Windows は動いているプロセスが開いているディレクトリを消せない）
pub struct Server {
    child: Child,
    log: PathBuf,
    pub port: u16,
}

impl Server {
    /// 自分の子プロセスが配信を始めるまで待つ。応答があるだけでは決めない = 空きを調べてから
    /// 子プロセスがバインドするまでの間（初回のビルド中）に、ほかのテストのサーバが同じポートで
    /// 答えることがある。目印を見て別のプロセスの応答を区別し、子プロセスがポートを取れずに
    /// 終わるのを待ってやり直す
    fn wait_ready(&mut self, marker: &str) -> Ready {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                let log = self.log();
                if log.contains("は既に使用中です") {
                    return Ready::PortInUse;
                }
                panic!("setoka が配信を始める前に終了しました（{status}）\n{log}");
            }
            let ours = self
                .get(MARKER_PATH)
                .is_some_and(|page| page.status == 200 && page.body.contains(marker));
            if ours {
                return Ready::Yes;
            }
            assert!(
                start.elapsed() < DEADLINE,
                "{} 秒待っても配信が始まりません\n{}",
                DEADLINE.as_secs(),
                self.log()
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// 標準出力と標準エラー出力（ログ）
    pub fn log(&self) -> String {
        fs::read_to_string(&self.log).unwrap_or_default()
    }

    pub fn get(&self, path: &str) -> Option<Response> {
        http_get(self.port, path)
    }

    /// `cond` が真になるまで待つ。上限を過ぎたらログ付きで失敗にする
    /// （再ビルドの完了を固定の時間で待たない。ログの文言でも判定しない）
    pub fn wait_until(&self, what: &str, mut cond: impl FnMut() -> bool) {
        let start = Instant::now();
        while !cond() {
            assert!(
                start.elapsed() < DEADLINE,
                "{} 秒待っても{what}\n--- setoka のログ\n{}",
                DEADLINE.as_secs(),
                self.log()
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// 空いているポート（OS に選ばせてすぐ閉じる）
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

pub struct Response {
    pub status: u16,
    /// ステータス行とヘッダ（改行は CRLF のまま）
    pub head: String,
    pub body: String,
}

impl Response {
    /// ヘッダの値（名前は大文字小文字を区別しない）
    pub fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim().eq_ignore_ascii_case(name).then(|| value.trim())
        })
    }
}

/// HTTP/1.1 の GET（`Connection: close` で、閉じられるまで読む）。繋がらなければ None
pub fn http_get(port: u16, path: &str) -> Option<Response> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .ok()?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n")?;
    let status = head.split_whitespace().nth(1)?.parse().ok()?;
    Some(Response {
        status,
        head: head.to_string(),
        body: body.to_string(),
    })
}
