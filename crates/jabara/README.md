# jabara 🍋

静的サイトの**開発サーバ**の部品をまとめた Rust ライブラリ。ビルドした静的サイトのディレクトリを配信し、
ファイルの監視と WebSocket のライブリロードで「編集 → 再ビルド → ブラウザの再読み込み」をつなぐ。
再ビルドそのものは使う側がコールバックで渡す（この crate はビルドの中身を知らない）。
名前は柑橘のじゃばらから。

- **配信**（`serve`。axum + tower-http の `ServeDir`）
  - `guide/` → `guide/index.html` を解決する。`base_url` が `/docs/` ならその下で配信し、`/` からはリダイレクトする
  - 無いページは `404.html` があればそれを 404 で返す（GitHub Pages と同じ）
  - 文字コードの無い `text/plain`・`text/markdown` に `charset=utf-8` を付ける（HTML・CSS・JS には付けない）
- **ライブリロード**（`ReloadNotifier`）— WebSocket のエンドポイント（既定は `/__livereload`。`base_url` に依らず
  オリジンのルートから）を生やし、`notify()` で接続中のページへ `reload` を送る
- **Host / Origin の検査** — DNS リバインディング対策。`localhost`・`*.localhost`・IP アドレスと、
  `allowed_hosts` に書いたホスト名だけを通す（Vite の `server.allowedHosts` と同じ考え方）。
  WebSocket は Origin も照合する。断った要求は 403 で、通す方法の案内（`forbidden_hint`）を添える
- **シンボリックリンクの遮断**（`PathGuard`）— `ServeDir` はパスを字句で検査するだけでリンクを辿るので、
  ファイルを開く前に呼ぶ述語を渡せる（拒否したパスは無いものとして 404）
- **ファイルの監視**（`watch`。notify）
  - 内容を変えない種類のイベント（開いた・読んだだけ）は捨てる（Linux の inotify は読んだだけでも届けるので、
    捨てないとビルドが原稿を読むたびに再ビルドが続く）
  - 静かになるまで（`debounce` の間イベントが来なくなるまで）集めてから、変わったパスの一覧でコールバックを呼ぶ
  - `WatchIgnore` で出力ディレクトリ（必ず除外する。除外しないと再ビルドが続く）・隠しディレクトリ
    （監視のルートからの相対パスで判定）・任意の述語を除外する
  - コールバックが panic したら監視を止め、配信も止める（`ServerError::WatchStopped`）

文言（エラー・ログ・403 の応答）は日本語。開発は [setoka](https://github.com/ai-implementer/setoka) の
monorepo で行っている（`setoka dev` / `preview` / `build --watch` がこれを使う）が、`setoka-*` には依存しない
独立のライブラリ。

## インストール

```bash
cargo add jabara
```

## 使い方

```rust
use std::net::IpAddr;
use std::time::Duration;

use jabara::{ReloadNotifier, ServeOptions, WatchIgnore};

let root = std::env::current_dir()?;
let notifier = ReloadNotifier::new();

// 原稿を監視して再ビルドし、成功したらページを再読み込みさせる（出力ディレクトリは必ず除外する）
let ignore = WatchIgnore::new(vec![root.join("dist")]);
let reload = notifier.clone();
let mut watcher = jabara::watch(&[root.clone()], ignore, Duration::from_millis(300), move |changed| {
    println!("変更: {changed:?}");
    // ここで再ビルドする。成功したときだけ通知する（失敗で通知すると壊れたページを読ませる）
    reload.notify();
})?;

// dist/ を配信する（ブロッキング。Ctrl+C で終わる）
let mut opts = ServeOptions::new(root.join("dist"), IpAddr::from([127, 0, 0, 1]), 5173);
opts.live_reload = Some(notifier);
opts.watch_failure = watcher.take_failure(); // 監視が止まったら配信も止める
jabara::serve(opts)?;
```

ページ側では、ライブリロードの経路に WebSocket で繋ぎ、`reload` を受けたら再読み込みする:

```html
<script>
  (function connect() {
    var ws = new WebSocket((location.protocol === "https:" ? "wss://" : "ws://") + location.host + "/__livereload");
    ws.onmessage = function () { location.reload(); };
    ws.onclose = function () { setTimeout(connect, 1000); };
  })();
</script>
```

- `ServeOptions::new(dir, host, port)` で作り、`base_url`・`live_reload`・`live_reload_path`・`path_guard`・
  `allowed_hosts`・`forbidden_hint`・`watch_failure` を必要に応じて書き換える（項目は増えることがあるので、
  構造体のリテラルでは作れない）
- `serve` は内部で tokio のランタイムを立ち上げるブロッキングの関数（呼び出し側を async にしなくてよい）
- ポートが使用中なら `ServerError::PortInUse`（別のポートの指定の仕方は使う側で案内する）

## ライセンス

MIT または Apache-2.0
