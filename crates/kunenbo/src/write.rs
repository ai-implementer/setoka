//! 書き込み・作成・削除（どれも経路の検査を通してから行う）

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::guard::{
    ensure_no_symlink_above, ensure_no_symlink_under, resolve_output_rel,
    without_trailing_separator,
};

/// 書き込みの結果（`Unchanged` = 内容が同じなので書かなかった）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    Written,
    Unchanged,
}

/// 内容が同じなら書き込みを省く（更新時刻を変えない）。
///
/// ⚠️ 経路の検証をしないので、**出力ディレクトリへ書くときは直接使わない**
/// （[`write_under`] を通すこと）。単体で使ってよいのは、パスを利用者の入力や記録から
/// 組み立てない書き込みだけ
pub fn write_if_changed(path: &Path, data: &[u8]) -> std::io::Result<WriteOutcome> {
    if let Ok(meta) = fs::metadata(path)
        && meta.len() == data.len() as u64
        && fs::read(path)? == data
    {
        return Ok(WriteOutcome::Unchanged);
    }
    fs::write(path, data)?;
    Ok(WriteOutcome::Written)
}

/// `root` の下の `rel`（`/` 区切り）へ安全に書き出す。解決したパスを返す。
///
/// **出力ディレクトリへ書くコードはすべてこれを通すこと。** 3 つの防御を
/// 1 実装にまとめている:
///
/// 1. `rel` の字句検証（[`resolve_output_rel`]。`..` や絶対パスを拒否）
/// 2. **最終パスまでの経路のリンク検査**（[`ensure_no_symlink_under`]）。
///    出力ルートまでを検査しても、`dist/guide -> /outside` のような
///    **出力ツリー内部**のリンクは別途見ないと書き込みが外へ抜ける
/// 3. 親ディレクトリの作成 ＋ 内容が同じなら書かない（[`write_if_changed`]。更新時刻を変えない）
///
/// `.kunenbo-tmp-` に数字が続く名前（大文字小文字は区別しない）を含む `rel` は断る（[`write_atomic_under`] が
/// 一時ディレクトリに使う名前。書き込み中の一時ファイルを別の書き込みが上書き・置き換えできないようにする）
pub fn write_under(
    root: &Path,
    rel: &str,
    data: &[u8],
) -> std::io::Result<(PathBuf, WriteOutcome)> {
    let path = resolve_output_rel(root, rel)?;
    reject_temp_name(rel)?;
    ensure_no_symlink_under(root, &path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let outcome = write_if_changed(&path, data)?;
    Ok((path, outcome))
}

/// [`write_under`] の原子的版（同じディレクトリに作った一時ディレクトリの中へ書いて、rename で置き換える）。
///
/// 使いどころは「**書き込みの途中で止まると、次に読む側が壊れた内容を
/// 掴む**」ファイルだけ（キャッシュの目次のように、それが健全なら残りも整合しているとみなす
/// ファイルなど）。
///
/// 3 つの防御（字句検証・経路のリンク検査・親ディレクトリの作成）は [`write_under`]
/// と同じものを通す。一時ファイルは、出力先と同じディレクトリに**排他的に**作った一時ディレクトリ
/// （`.kunenbo-tmp-{n}`。既にある名前は、ファイルでもリンクでも使わずに次の名前へ進む）の
/// 中に作り、書き終えたら出力先へ rename して、一時ディレクトリを消す。この名前（`.kunenbo-tmp-` に数字が続くもの。
/// 大文字小文字は区別しない）は一時ディレクトリ専用で、それを含む `rel` は [`write_under`] とこの関数が断る。
///
/// - 固定名の一時ファイルへ書くと、たまたま同じ名前の別のファイルを上書きして消し、そこに置かれたリンクを辿る
/// - 一時名に出力のファイル名を含めないので、名前の長さの上限に近い出力でも書ける
/// - 一時ファイルを同じディレクトリに直に置くと、同時に動く別の書き込みがその名前を出力先にしたとき、rename で
///   一時ファイルを置き換えられ、どちらも成功したまま中身が入れ替わる。一時ディレクトリの名前とその下は出力先に
///   できないので、別の書き込みが書き込み中の一時ファイルを置き換えることはない（PR #112 の再レビューの指摘）
///
/// 書き込みか置き換えに失敗したら、作った一時ファイルと一時ディレクトリを消す。
/// 内容が同じでも書き直す（原子性のため）ので、更新時刻を変えたくない出力には使わない
pub fn write_atomic_under(root: &Path, rel: &str, data: &[u8]) -> std::io::Result<PathBuf> {
    let path = resolve_output_rel(root, rel)?;
    reject_temp_name(rel)?;
    ensure_no_symlink_under(root, &path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // 同一ディレクトリへ置く（rename が同一ファイルシステム内で完結する）
    let tmp_dir = create_temp_dir_beside(&path)?;
    let tmp = tmp_dir.join("data");
    let written = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        file.write_all(data)?;
        // Windows は開いたままのファイルを rename できないので先に閉じる
        drop(file);
        fs::rename(&tmp, &path)
    })();
    // 消すのは自分で作った一時ファイルと一時ディレクトリだけ（rename できていれば一時ファイルはもう無い）
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    let _ = fs::remove_dir(&tmp_dir);
    written.map(|()| path)
}

/// `path` と同じディレクトリに一時ディレクトリを排他的に作る（[`write_atomic_under`] 用）
fn create_temp_dir_beside(path: &Path) -> std::io::Result<PathBuf> {
    // 出力先はこの名前を含まない（write_atomic_under が reject_temp_name で断っている）
    for n in 0..100 {
        let dir = path.with_file_name(format!(".kunenbo-tmp-{n}"));
        // create_dir は既にある名前（ファイル・ディレクトリ・リンク）では AlreadyExists で失敗し、リンクを辿らない
        match fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "使える一時ディレクトリの名前がありません（同じ名前のものが 100 個あります）",
    ))
}

/// `rel` が一時ディレクトリの名前（`.kunenbo-tmp-` に数字が続くもの）を含んでいれば断る。大文字小文字を
/// 区別しない FS も考えて、大文字小文字を畳んで比べる（[`write_under`]・[`write_atomic_under`] 用）
fn reject_temp_name(rel: &str) -> std::io::Result<()> {
    let reserved = rel.split('/').any(|seg| {
        seg.to_lowercase()
            .strip_prefix(".kunenbo-tmp-")
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    });
    if reserved {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "出力先として使えない相対パスです（.kunenbo-tmp- に数字が続く名前は一時ディレクトリに使います）: {rel}"
            ),
        ));
    }
    Ok(())
}

/// [`ensure_no_symlink_under`] で経路を確認してから、`dir` までのディレクトリを作る。
/// `dir` は `root` の下（`root` 自身は断る）
pub fn create_dir_all_under(root: &Path, dir: &Path) -> std::io::Result<()> {
    ensure_no_symlink_under(root, dir)?;
    fs::create_dir_all(dir)
}

/// [`ensure_no_symlink_under`] で経路を確認してからディレクトリを再帰削除する。
/// 削除したら `Ok(true)`、`dir` が無ければ何もせず `Ok(false)`。
///
/// 設定の字句検証（絶対パス・`..`・ほかのディレクトリとの重なり）をすり抜けた場合の最後の防波堤
/// （字句の検証はシンボリックリンクの存在を見られない）。`dir` 自身がリンクなら断る
/// （リンク先の中身を消さない）
pub fn remove_dir_all_under(root: &Path, dir: &Path) -> std::io::Result<bool> {
    ensure_no_symlink_under(root, dir)?;
    if !dir.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(dir)?;
    Ok(true)
}

/// 親ディレクトリまでの経路を確認してからファイルを消す。消したら `Ok(true)`、無ければ `Ok(false)`。
///
/// `root` 自身・`root` の外・`..` を含むパス・親までの経路のシンボリックリンクは断る。`path` 自身は
/// シンボリックリンクでもよく、**リンク自体を消す**（リンク先は辿らない）。要らなくなったファイルの掃除で、
/// 置かれたリンクを残すと、後で同じ名前へ書く [`write_under`] がそのリンクで断られ続けるため
/// （PR #112 のレビュー指摘）。前回の一覧から消す [`remove_orphans`](crate::remove_orphans) は、末端の
/// リンクも消さずに飛ばす（一覧は書き換えられうる記録なので、名前を当てたリンクを消させない）
pub fn remove_file_under(root: &Path, path: &Path) -> std::io::Result<bool> {
    ensure_no_symlink_above(root, path)?;
    // 末尾の区切り（`x/`）を落とす。付いたままだと remove_file がリンクを辿る
    let path = without_trailing_separator(path);
    match fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_if_changed_は同一内容でスキップし_mtime_を温存する() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        assert_eq!(
            write_if_changed(&path, b"hello").unwrap(),
            WriteOutcome::Written
        );
        let mtime1 = fs::metadata(&path).unwrap().modified().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(
            write_if_changed(&path, b"hello").unwrap(),
            WriteOutcome::Unchanged
        );
        let mtime2 = fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(mtime1, mtime2, "スキップ時は mtime が変わらない");

        assert_eq!(
            write_if_changed(&path, b"world").unwrap(),
            WriteOutcome::Written
        );
        assert_eq!(fs::read(&path).unwrap(), b"world");
    }

    #[test]
    fn write_under_は親ディレクトリを作って書き_同じ内容なら書かない() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("dist");
        let (path, outcome) = write_under(&root, "a/b.html", b"x").unwrap();
        assert_eq!(path, root.join("a").join("b.html"));
        assert_eq!(outcome, WriteOutcome::Written);
        assert_eq!(fs::read(&path).unwrap(), b"x");
        assert_eq!(
            write_under(&root, "a/b.html", b"x").unwrap().1,
            WriteOutcome::Unchanged
        );
        assert!(write_under(&root, "../outside.html", b"x").is_err());
        assert!(!dir.path().join("outside.html").exists());
    }

    /// 出力ツリー**内部**のリンク。ルートまでを検査しても、
    /// `dist/guide -> /outside` があると書き込みがリンク先へ抜ける
    #[cfg(unix)]
    #[test]
    fn write_under_は出力ツリー内部のリンクを拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("dist");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("guide")).unwrap();

        let err = write_under(&root, "guide/index.html", b"x").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(!outside.join("index.html").exists(), "リンク先へ書かない");

        // 出力ファイル自体がリンクの場合も同じ
        std::os::unix::fs::symlink(outside.join("t.html"), root.join("t.html")).unwrap();
        assert!(write_under(&root, "t.html", b"x").is_err());
        assert!(!outside.join("t.html").exists());
    }

    #[cfg(unix)]
    #[test]
    fn write_under_は基点自身がリンクなら拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        let root = dir.path().join("dist");
        std::os::unix::fs::symlink(&outside, &root).unwrap();

        assert!(write_under(&root, "index.html", b"x").is_err());
        assert!(!outside.join("index.html").exists(), "リンク先へ書かない");
    }

    #[test]
    fn write_atomic_under_は_tmp_を残さない() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_atomic_under(dir.path(), "cache/global.json", b"{}").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"{}");
        // 中間ファイルが残っていない（次回の読み込みや掃除が拾わない）
        assert_eq!(names(&dir.path().join("cache")), ["global.json"]);
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// 一時ファイルの名前に当たる既存のファイルを上書きしない（PR #112 のレビュー指摘。以前は
    /// `{rel}.tmp` の固定名へ書いて rename していたので、`report.json.tmp` が消えた）
    #[test]
    fn write_atomic_under_は同じディレクトリの別のファイルを消さない() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("report.json.tmp"), b"keep1").unwrap();
        fs::write(root.join(".kunenbo-tmp-0"), b"keep2").unwrap();

        write_atomic_under(root, "report.json", b"new").unwrap();
        assert_eq!(fs::read(root.join("report.json")).unwrap(), b"new");
        assert_eq!(fs::read(root.join("report.json.tmp")).unwrap(), b"keep1");
        assert_eq!(fs::read(root.join(".kunenbo-tmp-0")).unwrap(), b"keep2");
        assert_eq!(
            names(root),
            [".kunenbo-tmp-0", "report.json", "report.json.tmp"],
            "使った一時ファイルは残らない"
        );
    }

    /// 一時ファイルの名前に置かれたリンクを辿って、リンク先へ書かない
    #[cfg(unix)]
    #[test]
    fn write_atomic_under_は一時ファイルの名前のリンクを辿らない() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cache");
        let outside = dir.path().join("outside.json");
        fs::create_dir_all(&root).unwrap();
        fs::write(&outside, b"keep").unwrap();
        std::os::unix::fs::symlink(&outside, root.join(".kunenbo-tmp-0")).unwrap();

        write_atomic_under(&root, "global.json", b"new").unwrap();
        assert_eq!(fs::read(root.join("global.json")).unwrap(), b"new");
        assert_eq!(fs::read(&outside).unwrap(), b"keep", "リンク先へ書かない");
    }

    /// 一時ディレクトリの名前（`.kunenbo-tmp-` に数字が続くもの。大文字小文字は区別しない）とその下は出力先にできない
    /// （PR #112 の再レビューの指摘）
    #[test]
    fn 一時ディレクトリの名前とその下は出力先にできない() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for rel in [
            ".kunenbo-tmp-0",
            ".KUNENBO-TMP-12",
            ".kunenbo-tmp-0/data",
            "a/.kunenbo-tmp-3/b.json",
        ] {
            let err = write_atomic_under(root, rel, b"x").unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput, "{rel}");
            let err = write_under(root, rel, b"x").unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput, "{rel}");
        }
        assert!(names(root).is_empty(), "何も作らない");
        // 数字が続かないものは普通の名前
        for rel in [
            ".kunenbo-tmp-",
            ".kunenbo-tmp-x",
            ".kunenbo-tmp-0a",
            "kunenbo-tmp-0",
        ] {
            write_atomic_under(root, rel, b"x").unwrap();
            write_under(root, rel, b"y").unwrap();
        }
    }

    /// 書き込みの途中（一時ディレクトリ `.kunenbo-tmp-0` に中身がある）に、別の書き込みがその名前やその下を出力先に
    /// しても、途中の一時ファイルを置き換えずにエラーになる（PR #112 の再レビューの指摘。一時ファイルを直に置いて
    /// いたとき・一時ディレクトリの下を出力先にできたときは、どちらも成功したまま中身が入れ替わった）
    #[test]
    fn 書き込み中の一時ディレクトリは別の書き込みに置き換えられない() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // 書き込み A の途中を再現する
        fs::create_dir(root.join(".kunenbo-tmp-0")).unwrap();
        fs::write(root.join(".kunenbo-tmp-0/data"), b"A").unwrap();

        // 書き込み B: 出力先が A の一時ディレクトリの名前か、その下
        for rel in [".kunenbo-tmp-0", ".kunenbo-tmp-0/data"] {
            assert!(write_atomic_under(root, rel, b"B").is_err(), "{rel}");
            assert!(write_under(root, rel, b"B").is_err(), "{rel}");
        }
        assert_eq!(fs::read(root.join(".kunenbo-tmp-0/data")).unwrap(), b"A");
        assert_eq!(
            names(root),
            [".kunenbo-tmp-0"],
            "B の一時ディレクトリは残らない"
        );
    }

    /// 2 つのスレッドで同時に書く（A は `report.json`、B は A が使う一時ディレクトリの下）。B は必ず断られ、
    /// `report.json` には A の中身が入る（PR #112 の再レビューの指摘の回帰テスト）
    #[test]
    fn 同時に書いても一時ファイルを置き換えられない() {
        let a_data = vec![b'A'; 256 * 1024];
        for _ in 0..20 {
            let dir = tempfile::tempdir().unwrap();
            let barrier = std::sync::Barrier::new(2);
            let (a, b) = std::thread::scope(|s| {
                let a = s.spawn(|| {
                    barrier.wait();
                    write_atomic_under(dir.path(), "report.json", &a_data)
                });
                let b = s.spawn(|| {
                    barrier.wait();
                    write_atomic_under(dir.path(), ".kunenbo-tmp-0/data", b"B")
                });
                (a.join().unwrap(), b.join().unwrap())
            });
            a.unwrap();
            assert!(b.is_err());
            assert_eq!(fs::read(dir.path().join("report.json")).unwrap(), a_data);
            assert_eq!(names(dir.path()), ["report.json"]);
        }
    }

    /// 一時ファイルの名前が出力のファイル名に依らないので、上限（多くの FS で 255 バイト）に近い名前でも書ける
    /// （PR #112 の再レビューの指摘。出力のファイル名を含めた一時名だと上限を超えた）
    #[test]
    fn write_atomic_under_は長いファイル名でも書ける() {
        let dir = tempfile::tempdir().unwrap();
        let name = format!("{}.json", "a".repeat(245));
        write_atomic_under(dir.path(), &name, b"x").unwrap();
        assert_eq!(names(dir.path()), [name]);
    }

    #[test]
    fn write_atomic_under_は既存を置き換える() {
        let dir = tempfile::tempdir().unwrap();
        write_atomic_under(dir.path(), "a.json", b"old").unwrap();
        let path = write_atomic_under(dir.path(), "a.json", b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
    }

    #[test]
    fn write_atomic_under_も経路を検証する() {
        let dir = tempfile::tempdir().unwrap();
        // ルート外・絶対パスは write_under と同じ規律で拒否する
        assert!(write_atomic_under(dir.path(), "../outside.json", b"x").is_err());
        assert!(write_atomic_under(dir.path(), "/abs.json", b"x").is_err());
    }

    #[test]
    fn create_dir_all_under_はルートの下だけ作る() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cache");
        create_dir_all_under(&root, &root.join("pages/a")).unwrap();
        assert!(root.join("pages/a").is_dir());
        // 作成済みでも Ok
        create_dir_all_under(&root, &root.join("pages/a")).unwrap();
        assert!(create_dir_all_under(&root, &root).is_err(), "ルート自身");
        assert!(create_dir_all_under(&root, &dir.path().join("outside")).is_err());
        assert!(!dir.path().join("outside").exists());
    }

    /// `root/pages/../../outside` は前方一致では root の下に見える（PR #112 のレビュー指摘）
    #[test]
    fn 作成と削除は親ディレクトリ参照で外へ出ない() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cache");
        fs::create_dir_all(root.join("pages")).unwrap();
        fs::write(dir.path().join("keep.json"), b"keep").unwrap();
        let up = root.join("pages").join("..").join("..");

        assert!(create_dir_all_under(&root, &up.join("outside")).is_err());
        assert!(!dir.path().join("outside").exists(), "外に作らない");
        assert!(remove_file_under(&root, &up.join("keep.json")).is_err());
        assert!(dir.path().join("keep.json").exists(), "外を消さない");
        assert!(remove_dir_all_under(&root, &up.join("outside")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn create_dir_all_under_は経路のリンクを拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cache");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("pages")).unwrap();

        let err = create_dir_all_under(&root, &root.join("pages/a")).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(!outside.join("a").exists(), "リンク先に作らない");
    }

    #[test]
    fn remove_file_under_は消したかを返す() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.json"), b"x").unwrap();
        assert!(remove_file_under(root, &root.join("a.json")).unwrap());
        assert!(!root.join("a.json").exists());
        assert!(
            !remove_file_under(root, &root.join("a.json")).unwrap(),
            "無ければ false"
        );
        assert!(remove_file_under(root, root).is_err(), "ルート自身");
    }

    /// 末端のリンクはリンク自体を消す（リンク先は残る）。残すと後で同じ名前へ書けなくなる
    /// （PR #112 のレビュー指摘）
    #[cfg(unix)]
    #[test]
    fn remove_file_under_は末端のリンク自体を消す() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cache");
        let outside = dir.path().join("outside.json");
        fs::create_dir_all(root.join("pages")).unwrap();
        fs::write(&outside, b"keep").unwrap();
        let link = root.join("pages/stale.json");
        std::os::unix::fs::symlink(&outside, &link).unwrap();

        assert!(remove_file_under(&root, &link).unwrap());
        assert!(link.symlink_metadata().is_err(), "リンクは消える");
        assert_eq!(fs::read(&outside).unwrap(), b"keep", "リンク先は残る");
        write_under(&root, "pages/stale.json", b"new").unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn remove_file_under_は経路のリンクを拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cache");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("keep.json"), "守られるべきファイル".as_bytes()).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("pages")).unwrap();

        assert!(remove_file_under(&root, &root.join("pages/keep.json")).is_err());
        assert!(outside.join("keep.json").exists(), "リンク先は消さない");
    }

    #[test]
    fn remove_dir_all_under_はルート配下だけ削除する() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("dist");
        fs::create_dir_all(out.join("nested")).unwrap();
        fs::write(out.join("nested/a.html"), b"x").unwrap();

        assert!(remove_dir_all_under(dir.path(), &out).unwrap());
        assert!(!out.exists());
        // 不在なら何もせず false
        assert!(!remove_dir_all_under(dir.path(), &out).unwrap());
    }

    #[test]
    fn remove_dir_all_under_はルート自身を拒否する() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), b"").unwrap();

        let err = remove_dir_all_under(dir.path(), dir.path()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(dir.path().join("config.toml").exists());
    }

    /// canonicalize はリンクを追うため、ルート**内**を指すリンクは
    /// 「配下」の判定を通ってしまう（`dist -> content` で原稿が消える）
    #[cfg(unix)]
    #[test]
    fn remove_dir_all_under_はルート内を指すシンボリックリンクも拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        fs::create_dir_all(&content).unwrap();
        fs::write(content.join("index.md"), "守られるべきファイル".as_bytes()).unwrap();

        let link = dir.path().join("dist");
        std::os::unix::fs::symlink(&content, &link).unwrap();

        let err = remove_dir_all_under(dir.path(), &link).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(content.join("index.md").exists(), "原稿が残っている");
    }

    #[cfg(unix)]
    #[test]
    fn remove_dir_all_under_はシンボリックリンクの脱出を拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("proj");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("keep.txt"), "守られるべきファイル".as_bytes()).unwrap();

        let link = root.join("dist");
        std::os::unix::fs::symlink(&outside, &link).unwrap();

        let err = remove_dir_all_under(&root, &link).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(outside.join("keep.txt").exists());
    }
}
