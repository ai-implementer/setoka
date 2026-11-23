# kunenbo 🍊

出力ディレクトリへの**安全な書き込み**をまとめた Rust ライブラリ。静的サイトの生成器のように、利用者が書いた
設定や前回のビルドの記録からパスを組み立てて、決まったディレクトリの下へ書くプログラム向け。
名前は柑橘のくねんぼ（九年母）から。

- **相対パスの検証**（`resolve_output_rel`）— 絶対パス・`..`・`.`・空のセグメント・`\` を断る。`Path::join` は
  絶対パスを渡すと左辺を捨て、`.` はファイルシステムが吸収するので、文字列を比べるだけでは別のファイルの上書きを
  防げない
- **経路のシンボリックリンクの検査**（`ensure_no_symlink_under`）— 基点自身から対象までの各要素にリンクがあれば、
  向き先に関わらず断る（`dist/guide -> /outside` があると、書き込みも削除もリンク先へ抜ける。リンク先が基点の中でも
  別のディレクトリを壊せる）。基点の祖先は見ない（macOS の `/tmp -> private/tmp` のように、基点へ至る経路がリンク
  なのは普通のこと）。まだ無い要素は通す（これから作る）。読み側（配信）の版の `ensure_symlink_free` は基点自身を許す
- **書き込み**
  - `write_under` — 上の 2 つを通し、親ディレクトリを作ってから書く。内容が同じなら書かない（更新時刻を変えない）
  - `write_atomic_under` — 一時ファイル（`{rel}.tmp`）に書いてから rename で置き換える。途中で止まると次に読む側が
    壊れた内容を掴むファイル向け
- **作成と削除**（`create_dir_all_under`・`remove_dir_all_under`・`remove_file_under`）— 同じ検査を通してから作る・消す。
  基点自身は対象にしない
- **残ったファイルの掃除**（`remove_orphans`）— 前回書いて今回書かなかったファイルを消し、空になったディレクトリを
  剪定する
  - 前回の一覧は書き換えられうる記録から読むものとして扱い、使えないパス・経路にリンクがあるパスは消さずに飛ばして
    知らせる（`OrphanWarning::Skipped`）
  - **大文字小文字を区別しない FS**（macOS・Windows の既定）で `Guide/` を `guide/` に改名すると、今回の出力は前回の
    ファイルと同じファイルになる。文字列の差分で消すと今回の出力が消えるので、ファイルの同一性（Unix は dev / inode、
    Windows はファイル ID）で同じと分かったものは消さず、ディスク上の名前の大文字小文字を今回に合わせる。区別する FS
    （Linux 等）では別のファイルなので、前回の出力を消す
  - 出力をやめたディレクトリは丸ごと消さず、`list_files_under` で中を列挙して今回書かなかったものだけを消せる
    （利用者が同じ場所へ置いたファイルを消さない）

エラーの文言は日本語。開発は [setoka](https://github.com/ai-implementer/setoka) の monorepo で行っている（setoka の
`build` / `dev` が出力ディレクトリとキャッシュへの書き込みにこれを使う）が、`setoka-*` には依存しない独立のライブラリ。

## インストール

```bash
cargo add kunenbo
```

## 使い方

```rust
use std::collections::BTreeSet;

use kunenbo::{OrphanWarning, WriteOutcome};

let dist = std::env::current_dir()?.join("dist");

// 前回のビルドで書いたファイルの一覧（使う側が保存しておく）
let previous: BTreeSet<String> = ["index.html", "old/index.html"].map(String::from).into();

// 今回のビルド: 書いたパスを集める
let mut written = BTreeSet::new();
for (rel, html) in [("index.html", "<h1>home</h1>"), ("guide/index.html", "<h1>guide</h1>")] {
    let (_path, outcome) = kunenbo::write_under(&dist, rel, html.as_bytes())?;
    if outcome == WriteOutcome::Unchanged {
        // 内容が同じなので書かなかった（更新時刻はそのまま）
    }
    written.insert(rel.to_string());
}
// 利用者の入力から組み立てた外を指すパスは断る
assert!(kunenbo::write_under(&dist, "../outside.html", b"x").is_err());

// 前回あって今回無いもの（old/index.html）を消す。空になった old/ も消える
let removed = kunenbo::remove_orphans(&dist, &previous, &written, |warning: OrphanWarning| {
    eprintln!("警告: {warning}");
})?;
println!("{removed} 件を消しました");
```

- 相対パスは `/` 区切りの文字列で渡す（`&str`。OS の区切りに依らない）
- `write_if_changed` は経路の検査をしないので、利用者の入力からパスを組み立てる書き込みには使わない
- 検査と書き込みの間にほかのプロセスがリンクを置く競合（TOCTOU）は防がない。守るのは、設定や記録の書き間違い・
  置かれたままのリンクを辿って外へ書く・外を消す事故
- ログは出さない（掃除の警告はコールバックで渡す）

## ライセンス

MIT または Apache-2.0
