//! 前回書いて今回書かなかったファイルの掃除

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use crate::guard::{ensure_no_symlink_under, is_temp_name, reject_temp_name, resolve_output_rel};

/// 2 つのパスが同じファイルか（Unix は dev / inode、Windows はファイル ID。読めなければ別物）。
///
/// 大文字小文字を区別しない FS（macOS・Windows の既定）では、大文字小文字だけ違う 2 つのパスが
/// 同じファイルを指す。文字列を畳んで比べると区別する FS（Linux 等）で別のファイルを同じとみなすので、
/// ファイルの同一性で判定する
pub fn is_same_file(a: &Path, b: &Path) -> bool {
    same_file::is_same_file(a, b).unwrap_or(false)
}

/// [`remove_orphans`] が掃除を続けながら知らせること（掃除は止めない）
#[derive(Debug)]
#[non_exhaustive]
pub enum OrphanWarning {
    /// 前回の一覧の相対パスが使えない（[`resolve_output_rel`] が断った）か、経路にシンボリックリンクが
    /// ある（[`ensure_no_symlink_under`] が断った）か、一時ディレクトリ（`.kunenbo-tmp-{n}`）の下なので消さなかった
    Skipped { rel: String, error: std::io::Error },
    /// 今回と同じファイルだった前回のパスについて、ディスク上の名前の大文字小文字を今回に合わせられなかった
    /// （今回の出力は残っている。名前の大文字小文字が前回のままなだけ）
    CaseNotFixed { rel: String, error: std::io::Error },
}

impl std::fmt::Display for OrphanWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Skipped { rel, error } => {
                write!(f, "{rel} を消さずに飛ばしました: {error}")
            }
            Self::CaseNotFixed { rel, error } => {
                write!(f, "{rel} のパスの大文字小文字を付け直せません: {error}")
            }
        }
    }
}

/// `root` の下の `rel_dir` にある通常ファイルを、`root` 相対の `/` 区切りで列挙する。
/// `rel_dir` が無ければ空。シンボリックリンクは辿らず、リンク自体も挙げない。一時ディレクトリ（`.kunenbo-tmp-{n}`。
/// [`write_atomic_under`](crate::write_atomic_under) が書き込みの途中に使う）の中も挙げない。
///
/// [`remove_orphans`] に渡して「今回書かなかったものだけ消す」ために使う（出力をやめた
/// ディレクトリを丸ごと消すと、利用者が同じ場所へ置いたファイルまで消えてしまう）
pub fn list_files_under(root: &Path, rel_dir: &str) -> std::io::Result<BTreeSet<String>> {
    reject_temp_name(rel_dir)?;
    let dir = resolve_output_rel(root, rel_dir)?;
    ensure_no_symlink_under(root, &dir)?;
    let mut files = BTreeSet::new();
    if !dir.is_dir() {
        return Ok(files);
    }
    // 一時ディレクトリ（`.kunenbo-tmp-{n}`）の中は、同時に動く書き込みの途中のファイルなので挙げない
    // （挙げると remove_orphans が消し、その書き込みが NotFound で失敗する。PR #112 の再レビューの指摘）
    let walk = walkdir::WalkDir::new(&dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !is_temp_name(&e.file_name().to_string_lossy()));
    for entry in walk {
        let entry = entry.map_err(std::io::Error::other)?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .map_err(std::io::Error::other)?
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        files.insert(rel);
    }
    Ok(files)
}

/// 前回書き出したが今回書き出さなかったファイル（`previous` にあって `current` に無い相対パス）を
/// 削除し、空になったディレクトリを `root` の手前まで剪定する。削除件数を返す。
///
/// 前回の一覧は書き換えられうる記録から読むものとして、消す前にパスを検証する。使えない相対パスと、
/// 経路にシンボリックリンクがあるパス（`dist/guide -> /outside` を経由してリンク先を消さない）と、一時ディレクトリ
/// （`.kunenbo-tmp-{n}`）の下のパス（同時に動く書き込みの途中のファイルを消さない）は
/// 消さずに飛ばし、`on_warning` に [`OrphanWarning::Skipped`] を渡して続ける。
///
/// **大文字小文字を区別しない FS（macOS・Windows の既定）**: `Guide.md` を `guide.md` に
/// 改名すると、今回書いた `guide/index.html` は前回の `Guide/index.html` と同じファイルで、
/// 文字列の差分だけで消すと今回の出力が消える。前回のパスが今回書いたパスと
/// **同じファイルを指すときだけ**消さずに、ディスク上の名前の大文字小文字を今回に合わせて付け直す
/// （付け直せなければ [`OrphanWarning::CaseNotFixed`]）。
/// 判定は文字列ではなくファイルの同一性（[`is_same_file`]）で行う — 区別する FS（Linux 等）では
/// 2 つは別のファイルで、前回の出力を消すのが正しい。文字列の大文字小文字を無視して「一致したら
/// 消さない」とすると、前回の出力が残ったうえ次の一覧から落ちて二度と掃除されない。
/// 候補は大文字小文字を畳んだ文字列で探すので、Unicode の正規化（NFC / NFD）だけが
/// 違う改名は拾えない
pub fn remove_orphans(
    root: &Path,
    previous: &BTreeSet<String>,
    current: &BTreeSet<String>,
    on_warning: impl FnMut(OrphanWarning),
) -> std::io::Result<usize> {
    remove_orphans_with(root, previous, current, is_same_file, on_warning)
}

/// [`remove_orphans`] の本体。`same` は「2 つのパスが同じファイルか」（テストで
/// 大文字小文字を区別しない FS を模すために差し替える）
fn remove_orphans_with(
    root: &Path,
    previous: &BTreeSet<String>,
    current: &BTreeSet<String>,
    same: impl Fn(&Path, &Path) -> bool,
    mut on_warning: impl FnMut(OrphanWarning),
) -> std::io::Result<usize> {
    let mut removed = 0usize;
    let mut dirs: BTreeSet<PathBuf> = BTreeSet::new();
    // 今回書いたパスを、大文字小文字を畳んだキーで引く（同じファイルの候補探し）
    let mut folded: HashMap<String, Vec<&str>> = HashMap::new();
    for rel in current {
        folded.entry(rel.to_lowercase()).or_default().push(rel);
    }
    let mut case_fixes: Vec<&str> = Vec::new();
    for rel in previous.difference(current) {
        // 前回の一覧は書き換えられうるので、削除前に検証する。
        // 経路のリンク検査も込み（`dist/guide -> /outside` を経由して
        // リンク先のファイルを消さないため）。
        // 不正な rel は掃除を止めず読み飛ばす（掃除できないだけで害はない）
        // 一時ディレクトリ（`.kunenbo-tmp-{n}`）の下は、同時に動く書き込みの途中のファイルなので消さない
        let path = match reject_temp_name(rel)
            .and_then(|()| resolve_output_rel(root, rel))
            .and_then(|path| ensure_no_symlink_under(root, &path).map(|()| path))
        {
            Ok(path) => path,
            Err(error) => {
                on_warning(OrphanWarning::Skipped {
                    rel: rel.clone(),
                    error,
                });
                continue;
            }
        };
        let same_as_current = folded.get(&rel.to_lowercase()).and_then(|candidates| {
            candidates.iter().copied().find(|cur| {
                resolve_output_rel(root, cur).is_ok_and(|cur_path| same(&path, &cur_path))
            })
        });
        if let Some(cur) = same_as_current {
            case_fixes.push(cur);
            continue;
        }
        match fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let mut dir = path.parent();
        while let Some(d) = dir {
            if d == root {
                break;
            }
            dirs.insert(d.to_path_buf());
            dir = d.parent();
        }
    }
    // 深い順に空ディレクトリを剪定（非空は remove_dir が失敗するだけなので無視）
    for dir in dirs.iter().rev() {
        let _ = fs::remove_dir(dir);
    }
    for rel in case_fixes {
        if let Err(error) = match_case(root, rel) {
            on_warning(OrphanWarning::CaseNotFixed {
                rel: rel.to_string(),
                error,
            });
        }
    }
    Ok(removed)
}

/// `rel` の各構成要素について、ディスク上の名前が大文字小文字だけ違うなら今回の名前へ
/// 改名する（大文字小文字を区別しない FS で、既存の `Guide/` に書き込んだ出力を `guide/` に
/// する。区別するサーバへ上げると `/guide/` が 404 になるため）。同じ名前のエントリが
/// あれば何もしない。改名は一時名を経由する（大文字小文字だけの改名を 1 回の rename で
/// 受け付けない FS があるため）
fn match_case(root: &Path, rel: &str) -> std::io::Result<()> {
    let mut parent = root.to_path_buf();
    for name in rel.split('/') {
        let exact = parent.join(name);
        let mut on_disk = None;
        for entry in fs::read_dir(&parent)? {
            let entry_name = entry?.file_name();
            let entry_name = entry_name.to_string_lossy();
            if entry_name == name {
                on_disk = None;
                break;
            }
            if entry_name.to_lowercase() == name.to_lowercase() {
                on_disk = Some(entry_name.into_owned());
            }
        }
        if let Some(old) = on_disk {
            let old = parent.join(old);
            let tmp = unused_name(&parent)?;
            ensure_no_symlink_under(root, &old)?;
            fs::rename(&old, &tmp)?;
            fs::rename(&tmp, &exact)?;
        }
        parent = exact;
    }
    Ok(())
}

/// `dir` の中で、まだ何も無い一時名を選ぶ。rename は行き先に既にあるものを上書きするので、
/// 固定名だと、たまたま同じ名前の出力を消してしまう。区別しない FS では、大文字小文字違いの
/// 既存エントリも `symlink_metadata` が見つける。名前に元の名前を含めない（含めると、上限に近い長さの名前だけ
/// 付け直せなくなる）
fn unused_name(dir: &Path) -> std::io::Result<PathBuf> {
    for n in 0..100 {
        let candidate = dir.join(format!(".kunenbo-case-{n}"));
        match fs::symlink_metadata(&candidate) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(candidate),
            Err(e) => return Err(e),
            Ok(_) => continue,
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "改名に使える一時名がありません（同じ名前のエントリが 100 個あります）",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write::write_under;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn case_insensitive(a: &Path, b: &Path) -> bool {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    }

    #[test]
    fn remove_orphans_は差分削除と空ディレクトリ剪定をする() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for rel in ["index.html", "old/index.html", "keep/index.html"] {
            write_under(root, rel, b"x").unwrap();
        }
        let previous = set(&["index.html", "old/index.html", "keep/index.html"]);
        write_under(root, "keep/other.html", b"d").unwrap();
        let current = set(&["index.html", "keep/other.html"]);

        let mut warnings = Vec::new();
        let removed = remove_orphans(root, &previous, &current, |w| warnings.push(w)).unwrap();
        assert_eq!(removed, 2);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(!root.join("old").exists(), "空ディレクトリは剪定");
        assert!(root.join("keep").exists(), "非空ディレクトリは残す");
        assert!(root.join("index.html").exists());
    }

    /// 大文字小文字を区別する FS（Linux の CI）: `Guide/index.html` と `guide/index.html` は
    /// 別のファイルなので、改名前の旧出力は消える（同一性で判定するので文字列が似ていても残さない）
    #[test]
    fn 大文字小文字だけ違う旧出力は別のファイルなら消す() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("Guide")).unwrap();
        fs::write(root.join("Guide/index.html"), b"old").unwrap();
        // 区別しない FS では同じ場所になるので、この前提が成り立つ環境だけで見る
        if root.join("guide/index.html").exists() {
            return;
        }
        fs::create_dir_all(root.join("guide")).unwrap();
        fs::write(root.join("guide/index.html"), b"new").unwrap();
        let previous = set(&["Guide/index.html"]);
        let current = set(&["guide/index.html"]);

        assert_eq!(
            remove_orphans(root, &previous, &current, |_| {}).unwrap(),
            1
        );
        assert!(!root.join("Guide").exists(), "旧出力とディレクトリは消える");
        assert_eq!(fs::read(root.join("guide/index.html")).unwrap(), b"new");
    }

    /// 大文字小文字を区別しない FS（macOS・Windows の既定）: 今回の `guide/index.html` は既存の
    /// `Guide/` に書かれて旧出力と同じファイルになる。消さずに、パスを今回の大文字小文字へ
    /// 付け直す。Linux では同一性の判定を差し替えて、その FS を模す
    #[test]
    fn 旧出力が今回と同じファイルなら消さずに大文字小文字を付け直す() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("Guide")).unwrap();
        fs::write(root.join("Guide/index.html"), b"new").unwrap();
        // 改名の一時名と同じ名前の出力（利用者のファイルのコピー等）があっても上書きしない
        fs::write(root.join(".kunenbo-case-0"), b"keep").unwrap();
        let previous = set(&["Guide/index.html"]);
        let current = set(&["guide/index.html", ".kunenbo-case-0"]);

        let mut warnings = Vec::new();
        let removed = remove_orphans_with(root, &previous, &current, case_insensitive, |w| {
            warnings.push(w)
        })
        .unwrap();
        assert_eq!(removed, 0, "今回の出力を消さない");
        assert!(warnings.is_empty(), "{warnings:?}");
        let mut names: Vec<String> = fs::read_dir(root)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [".kunenbo-case-0", "guide"],
            "ディレクトリ名を今回の大文字小文字に付け直し、一時名を残さない"
        );
        assert_eq!(fs::read(root.join("guide/index.html")).unwrap(), b"new");
        assert_eq!(fs::read(root.join(".kunenbo-case-0")).unwrap(), b"keep");
    }

    /// 一時名に元の名前を含めないので、上限（多くの FS で 255 バイト）に近い名前でも付け直せる
    #[test]
    fn 長い名前でも大文字小文字を付け直せる() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let old = format!("G{}", "a".repeat(249));
        let new = format!("g{}", "a".repeat(249));
        fs::create_dir_all(root.join(&old)).unwrap();
        fs::write(root.join(&old).join("index.html"), b"new").unwrap();
        let previous = set(&[&format!("{old}/index.html")]);
        let current = set(&[&format!("{new}/index.html")]);

        let mut warnings = Vec::new();
        remove_orphans_with(root, &previous, &current, case_insensitive, |w| {
            warnings.push(w)
        })
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        // 大文字小文字を区別しない FS（macOS・Windows）では旧表記でも exists() が true になるので、
        // ディレクトリの列挙で実際の名前を見る（PR #112 の再レビューの指摘）
        let names: Vec<String> = fs::read_dir(root)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, std::slice::from_ref(&new));
        assert!(root.join(&new).join("index.html").is_file());
    }

    #[test]
    fn remove_orphans_は危険な相対パスを飛ばして続行する() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside.html");
        let root = dir.path().join("dist");
        fs::create_dir_all(&root).unwrap();
        fs::write(&outside, "守られるべきファイル".as_bytes()).unwrap();
        fs::write(root.join("stale.html"), b"x").unwrap();

        let previous = set(&["../outside.html", "stale.html"]);
        let mut warnings = Vec::new();
        let removed =
            remove_orphans(&root, &previous, &BTreeSet::new(), |w| warnings.push(w)).unwrap();

        assert_eq!(removed, 1, "不正な rel は数えない");
        assert!(outside.exists(), "出力先の外は消さない");
        assert!(!root.join("stale.html").exists());
        assert!(
            matches!(
                warnings.as_slice(),
                [OrphanWarning::Skipped { rel, error }]
                    if rel == "../outside.html" && error.kind() == std::io::ErrorKind::InvalidInput
            ),
            "{warnings:?}"
        );
    }

    /// 掃除もリンク経由でリンク先のファイルを消してはいけない
    #[cfg(unix)]
    #[test]
    fn remove_orphans_は内部リンク経由の削除をしない() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("dist");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("keep.html"), "守られるべきファイル".as_bytes()).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("guide")).unwrap();

        let previous = set(&["guide/keep.html"]);
        let mut warnings = Vec::new();
        let removed =
            remove_orphans(&root, &previous, &BTreeSet::new(), |w| warnings.push(w)).unwrap();

        assert_eq!(removed, 0);
        assert!(outside.join("keep.html").exists(), "リンク先は消さない");
        assert!(
            matches!(warnings.as_slice(), [OrphanWarning::Skipped { rel, .. }] if rel == "guide/keep.html"),
            "{warnings:?}"
        );
    }

    #[test]
    fn list_files_under_は配下の通常ファイルを列挙する() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_under(root, "_search/index/a.bin", b"a").unwrap();
        write_under(root, "_search/manifest.json", b"{}").unwrap();
        write_under(root, "index.html", b"x").unwrap();

        assert_eq!(
            list_files_under(root, "_search").unwrap(),
            set(&["_search/index/a.bin", "_search/manifest.json"])
        );
        assert!(list_files_under(root, "missing").unwrap().is_empty());
        assert!(list_files_under(root, "../x").is_err());
    }

    /// 書き込みの途中の一時ディレクトリ（`.kunenbo-tmp-{n}`）の中は列挙せず、掃除でも消さない。列挙して掃除に渡すと、
    /// 同時に動く書き込みの一時ファイルが消え、その書き込みが NotFound で失敗した（PR #112 の再レビューの指摘）
    #[test]
    fn 一時ディレクトリの中は列挙せず掃除でも消さない() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_under(root, "_search/manifest.json", b"{}").unwrap();
        // 書き込みの途中を再現する（Windows で同じものを指す末尾のピリオドの表記も）
        for tmp in ["_search/.kunenbo-tmp-0", "_search/index/.KUNENBO-TMP-1."] {
            fs::create_dir_all(root.join(tmp)).unwrap();
            fs::write(root.join(tmp).join("data"), b"A").unwrap();
        }

        let listed = list_files_under(root, "_search").unwrap();
        assert_eq!(listed, set(&["_search/manifest.json"]));
        assert!(list_files_under(root, "_search/.kunenbo-tmp-0").is_err());

        // 前回の一覧に一時ディレクトリの下が入っていても消さずに飛ばす
        let previous = set(&[
            "_search/manifest.json",
            "_search/.kunenbo-tmp-0/data",
            "_search/index/.KUNENBO-TMP-1./data",
        ]);
        let mut warnings = Vec::new();
        let removed =
            remove_orphans(root, &previous, &BTreeSet::new(), |w| warnings.push(w)).unwrap();
        assert_eq!(removed, 1, "manifest.json だけ");
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert_eq!(
            fs::read(root.join("_search/.kunenbo-tmp-0/data")).unwrap(),
            b"A"
        );
        assert_eq!(
            fs::read(root.join("_search/index/.KUNENBO-TMP-1./data")).unwrap(),
            b"A"
        );
    }

    #[test]
    fn 警告の文言() {
        let w = OrphanWarning::Skipped {
            rel: "a.html".into(),
            error: std::io::Error::other("理由"),
        };
        assert_eq!(w.to_string(), "a.html を消さずに飛ばしました: 理由");
    }
}
