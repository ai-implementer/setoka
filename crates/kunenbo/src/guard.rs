//! 相対パスの検証と、経路のシンボリックリンクの検査

use std::path::{Component, Path, PathBuf};

/// 出力先の相対パス `rel`（`/` 区切り）を検証して `root` に結合する。
///
/// 絶対パス・`..`・`.`・空のセグメント・`\` を含む `rel` を断る（`ErrorKind::InvalidInput`）。
/// 利用者の入力や前回のビルドの記録（書き換えられうるファイル）の文字列をそのまま `root.join()` すると、
/// `Path::join` は絶対パスを渡されると左辺を捨て、`.` はファイルシステムが吸収するので、文字列の一致を
/// 比べるだけでは別のファイルの上書きを防げない。判定をここ 1 か所にまとめる。
///
/// シンボリックリンクは見ない（字句だけの検査）。経路の検査は [`ensure_no_symlink_under`]
pub fn resolve_output_rel(root: &Path, rel: &str) -> std::io::Result<PathBuf> {
    let invalid = |reason: &str| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("出力先として使えない相対パスです（{reason}）: {rel}"),
        )
    };
    if rel.is_empty() {
        return Err(invalid("空です"));
    }
    if rel.contains('\\') {
        return Err(invalid("区切りは / を使ってください"));
    }
    let mut path = root.to_path_buf();
    for seg in rel.split('/') {
        match seg {
            "" => return Err(invalid("空のセグメントを含みます")),
            "." => return Err(invalid(". セグメントを含みます")),
            ".." => return Err(invalid(".. セグメントを含みます")),
            _ => {}
        }
        // ドライブレター等（Windows の Prefix / RootDir）を弾く。
        // 単一の通常セグメントに解決できるものだけ通す
        let mut comps = Path::new(seg).components();
        match (comps.next(), comps.next()) {
            (Some(Component::Normal(s)), None) => path.push(s),
            _ => return Err(invalid("パス要素として使えません")),
        }
    }
    Ok(path)
}

/// `root` から `target` までの**各パス要素**にシンボリックリンクが無く、
/// `target` が `root` の真の子孫であることを確認する。
/// まだ存在しない末端要素は許容する（これから作るため）。
///
/// 出力先への**書き込み・削除の前に必ず通す**こと。canonicalize による
/// 「ルート配下か」の判定だけでは足りない:
///
/// - `dist -> ../outside` は、出力先を消してから作り直さない（前回の出力に書き足す）ビルドだと
///   削除の経路を通らないので、書き込みだけが黙って外へ出る
/// - `alias -> <ルート>` の下に出力先（`alias/content`）を置くと、**リンク先がルート内**
///   なので配下の判定を通り、ルートの下の別のディレクトリを壊せる
/// - 最終要素だけ見る判定では中間要素のリンクを取りこぼす
///
/// リンクは**リンク先がどこでも拒否する**（ルート内を指していても危険なため）。
///
/// 検査範囲は `root` 自身と、そこから `target` までの各要素。
/// **root の祖先は見ない**（macOS の `/tmp -> private/tmp` のように、
/// 基点へ至る経路がリンクなのは正常なため）。
/// まだ存在しない要素は検査対象が無いので `Ok`（これから実体を作る）。
///
/// root 自身を含めるのは、呼び出し側が検証していない基点を渡せてしまうため。
/// 基点がリンクなら、その下をいくら検査しても書き込みは全部リンク先の中に入る。
///
/// `target == root` も断る（基点そのものを書き換え・削除の対象にしない）。読み側の版は
/// [`ensure_symlink_free`]
pub fn ensure_no_symlink_under(root: &Path, target: &Path) -> std::io::Result<()> {
    if target
        .strip_prefix(root)
        .is_ok_and(|rel| rel.as_os_str().is_empty())
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("出力先が基準ディレクトリ自身です: {}", root.display()),
        ));
    }
    ensure_symlink_free(root, target)
}

/// kunenbo が一時ディレクトリに使う名前（`.kunenbo-tmp-` に数字が続くもの）か。
///
/// Windows の通常のパスの解決（Win32）は、大文字小文字を区別せず、名前の末尾のピリオドと空白を落とす
/// （`.kunenbo-tmp-0.` と `.KUNENBO-TMP-0 ` は `.kunenbo-tmp-0` と同じものを指す）ので、同じように畳んでから比べる。
/// ほかの OS では別の名前だが、規則を 1 つにするために同じく一時ディレクトリの名前とみなす。
/// Windows の 8.3 形式の短い名前（`KUNENB~1`）や代替データストリーム（`名前:ストリーム`）で同じ
/// ディレクトリを指す書き方は見分けない（名前の文字列で判定しているため）
pub(crate) fn is_temp_name(seg: &str) -> bool {
    seg.trim_end_matches(['.', ' '])
        .to_lowercase()
        .strip_prefix(".kunenbo-tmp-")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// `rel` が一時ディレクトリの名前（[`is_temp_name`]）を含んでいれば断る（書き込み中の一時ファイルを、別の書き込みや
/// 掃除が上書き・置き換え・削除しないように。[`write_under`](crate::write_under)・
/// [`write_atomic_under`](crate::write_atomic_under)・[`remove_orphans`](crate::remove_orphans)・
/// [`list_files_under`](crate::list_files_under) 用）
pub(crate) fn reject_temp_name(rel: &str) -> std::io::Result<()> {
    if rel.split('/').any(is_temp_name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "出力先として使えない相対パスです（.kunenbo-tmp- に数字が続く名前は一時ディレクトリに使います）: {rel}"
            ),
        ));
    }
    Ok(())
}

/// 構成要素から組み直したパス（末尾の区切りを落とす。`dist/` → `dist`）。末尾に区切りがあると、
/// `symlink_metadata` や `remove_file` はリンクを辿ってリンク先を見る（POSIX のパスの解決）
pub(crate) fn without_trailing_separator(path: &Path) -> PathBuf {
    path.components().collect()
}

/// [`ensure_no_symlink_under`] の、末端の要素だけはシンボリックリンクでもよい版（リンク自体を消すときに使う。
/// `remove_file` はリンクを辿らずリンク自体を消すので、末端がリンクでもリンク先には届かない）。
/// `target == root`・root の外・`..` を含むパス・親までの経路のリンクは断る
pub(crate) fn ensure_no_symlink_above(root: &Path, target: &Path) -> std::io::Result<()> {
    let invalid = |message: String| std::io::Error::new(std::io::ErrorKind::InvalidInput, message);
    if target
        .strip_prefix(root)
        .is_ok_and(|rel| rel.as_os_str().is_empty())
    {
        return Err(invalid(format!(
            "出力先が基準ディレクトリ自身です: {}",
            root.display()
        )));
    }
    // `file_name` は末尾が `..` のとき None
    let parent = target
        .file_name()
        .and(target.parent())
        .ok_or_else(|| invalid(format!("{} はファイルを指していません", target.display())))?;
    // 親は root 自身でもよい。root の外・`..`・経路のリンクはここで断る
    ensure_symlink_free(root, parent)
}

/// `target` が `root` 自身またはその配下で、`root` から `target` までの経路に
/// シンボリックリンクが無いことを確認する。
///
/// [`ensure_no_symlink_under`] の読み側（配信）版で、違いは **`target == root` を
/// 許す**こと（開発サーバは `/` の要求で基点ディレクトリ自身の metadata を
/// 引く）。検査範囲と「まだ存在しない要素は Ok」の規律は書き側と同じ = 書き側が
/// 拒否したリンクを読み側だけが辿ることがない。
/// `root` 自身がリンクなら `Ok` になる経路は無い（配下をいくら検査しても無意味なため）。
/// `root` より後ろに `..` を含む `target`（`root/a/../../outside`）も断る（前方一致だけでは外を指せるため）
pub fn ensure_symlink_free(root: &Path, target: &Path) -> std::io::Result<()> {
    let invalid = |message: String| std::io::Error::new(std::io::ErrorKind::InvalidInput, message);

    let rel = target.strip_prefix(root).map_err(|_| {
        invalid(format!(
            "{} が {} の外を指しています",
            target.display(),
            root.display()
        ))
    })?;
    // strip_prefix は字句の前方一致を見るだけで、残りの `..` を畳まない（`root/a/../../outside` は
    // `a/../../outside` が残り、下の検査を通ったうえで root の外を指す）。ファイルシステムに触る前に断る
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(invalid(format!(
            "{} は .. を含みます（{} の外を指せるので使いません）",
            target.display(),
            root.display()
        )));
    }
    // 末尾に区切りがある `dist/` を渡すと、lstat はリンクを辿ってリンク先のディレクトリを返す（POSIX）ので、
    // 構成要素から組み直して末尾の区切りを落としてから見る（PR #112 の再レビューの指摘）
    let root = without_trailing_separator(root);
    if root
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err(invalid(format!(
            "基準ディレクトリ {} がシンボリックリンクです（リンク先へは読み書きしません）",
            root.display()
        )));
    }

    let mut cur = root;
    for comp in rel.components() {
        cur.push(comp);
        match cur.symlink_metadata() {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(invalid(format!(
                    "経路にシンボリックリンクがあります: {}（リンク先へは読み書きしません）",
                    cur.display()
                )));
            }
            Ok(_) => {}
            // ここから下はまだ存在しない = 書き側はこれから実ディレクトリを作り、
            // 読み側は素の NotFound になる
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::fs;

    #[test]
    fn resolve_output_rel_は危険な相対パスを拒否する() {
        let dir = tempfile::tempdir().unwrap();
        // `.` はファイルシステムが吸収するため、文字列比較では別物に見えるのに同じファイルを指す
        for bad in [
            "./index.html",
            "../outside.html",
            "a/./b.html",
            "a/../b.html",
            "/etc/passwd",
            "a//b.html",
            "a\\b.html",
            "",
        ] {
            let err = resolve_output_rel(dir.path(), bad).unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput, "{bad:?}");
        }
        assert_eq!(
            resolve_output_rel(dir.path(), "a/b.html").unwrap(),
            dir.path().join("a").join("b.html")
        );
    }

    #[cfg(unix)]
    #[test]
    fn ensure_no_symlink_under_は末端のリンクを拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("proj");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("dist")).unwrap();

        let err = ensure_no_symlink_under(&root, &root.join("dist")).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    /// 最終要素だけ見る判定では取りこぼす形。リンク先がルート**内**でも拒否する
    #[cfg(unix)]
    #[test]
    fn ensure_no_symlink_under_は中間要素のリンクを拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("content")).unwrap();
        std::os::unix::fs::symlink(root, root.join("alias")).unwrap();

        let err = ensure_no_symlink_under(root, &root.join("alias/content")).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn ensure_no_symlink_under_は未作成の末端を許容する() {
        let dir = tempfile::tempdir().unwrap();
        // dist も その下も まだ無い（これから作る）
        ensure_no_symlink_under(dir.path(), &dir.path().join("dist")).unwrap();
        ensure_no_symlink_under(dir.path(), &dir.path().join("a/b/c")).unwrap();
    }

    #[test]
    fn ensure_no_symlink_under_はルート自身とルート外を拒否する() {
        let dir = tempfile::tempdir().unwrap();
        assert!(ensure_no_symlink_under(dir.path(), dir.path()).is_err());
        assert!(ensure_no_symlink_under(dir.path(), Path::new("/etc")).is_err());
    }

    /// 前方一致では root の下に見えるが、`..` で外を指すパス（PR #112 のレビュー指摘）
    #[test]
    fn root_より後ろの親ディレクトリ参照を拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("dist");
        let real = root.join("a");
        let escape = real.join("..").join("..").join("outside");
        for check in [ensure_no_symlink_under, ensure_symlink_free] {
            let err = check(&root, &escape).unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
            assert!(check(&root, &root.join("a").join("..").join("b")).is_err());
        }
        let err = ensure_no_symlink_above(&root, &escape).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(ensure_no_symlink_above(&root, &real.join("..")).is_err());
    }

    /// 基点の末尾に区切りがあると、lstat はリンクを辿ってリンク先のディレクトリを返す（PR #112 の再レビューの指摘）
    #[cfg(unix)]
    #[test]
    fn 末尾に区切りがある基点でもリンクを拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        let link = dir.path().join("dist");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let mut with_slash = link.into_os_string();
        with_slash.push("/");
        let root = Path::new(&with_slash);

        assert!(ensure_no_symlink_under(root, &root.join("index.html")).is_err());
        assert!(ensure_symlink_free(root, root).is_err());
        assert!(ensure_no_symlink_above(root, &root.join("index.html")).is_err());
        assert!(crate::write_under(root, "index.html", b"x").is_err());
        assert!(!outside.join("index.html").exists(), "リンク先へ書かない");
    }

    #[test]
    fn ensure_no_symlink_above_は親まで検査し基点自身と外を拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cache");
        ensure_no_symlink_above(&root, &root.join("a.json")).unwrap();
        ensure_no_symlink_above(&root, &root.join("pages/a.json")).unwrap();
        assert!(ensure_no_symlink_above(&root, &root).is_err());
        assert!(ensure_no_symlink_above(&root, &dir.path().join("a.json")).is_err());
    }

    /// 読み側の版は基点自身を許す（基点の外とリンクは書き側と同じく断る）
    #[cfg(unix)]
    #[test]
    fn ensure_symlink_free_はルート自身を許しリンクは拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("dist");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("guide")).unwrap();

        ensure_symlink_free(&root, &root).unwrap();
        ensure_symlink_free(&root, &root.join("index.html")).unwrap();
        assert!(ensure_symlink_free(&root, &root.join("guide/index.html")).is_err());
        assert!(ensure_symlink_free(&root, &outside).is_err());
    }

    /// macOS の `/tmp -> private/tmp` 相当。ルートへ至る経路のリンクは
    /// 検査対象外にしないと、tempdir 上の全テストと実プロジェクトが落ちる
    #[cfg(unix)]
    #[test]
    fn ensure_no_symlink_under_は祖先のリンクを問題にしない() {
        // リンクは**基点の祖先**であって基点自身ではない。
        // 祖先まで検査すると tempdir 上の全テストと実プロジェクトが落ちる
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        fs::create_dir_all(real.join("proj/dist")).unwrap();
        let ancestor_link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &ancestor_link).unwrap();

        let root = ancestor_link.join("proj");
        ensure_no_symlink_under(&root, &root.join("dist")).unwrap();
    }

    #[test]
    fn ensure_no_symlink_under_はルート未作成でも通る() {
        // 記録やキャッシュの保存は「作る前」に検証するため
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("not-yet");
        ensure_no_symlink_under(&missing, &missing.join("a/b.json")).unwrap();
    }
}
