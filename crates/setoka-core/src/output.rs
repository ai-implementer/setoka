//! インクリメンタルビルドの出力トラッキング。
//!
//! - [`OutputTracker`] — このビルドで書き出した dist 相対パスと出どころ（[`OutputKind`]）を記録する
//! - [`remove_orphans`] — 前回マニフェストとの差分で、削除ページの古い出力だけ掃除する
//! - [`save_manifest`] / [`load_manifest`] — 前回書き出したパスの一覧（`.setoka/cache/`）
//!
//! **出力先への書き込みは [`write_under`]、削除は [`remove_dir_all_under`] を必ず通す。**
//! どちらも [`resolve_output_rel`]（rel の字句検証）と
//! [`ensure_no_symlink_under`]（経路のリンク検査）を内包していて、
//! `root.join(rel)` を直に書くとこの 2 つが抜ける。
//! リンク検査は**出力ツリーの内部**（`dist/guide -> /outside`）まで見る必要があり、
//! 出力ルートまでの検査だけでは書き込み・削除がリンク先へ抜ける。
//!
//! 書き込みの関数の実体は crate kunenbo（v0.22 Phase 106 で切り出した）。setoka の crate はここの
//! 再エクスポートを通して使う（ワークスペースの `clippy.toml` が `std::fs` の書き込み・削除・作成を禁じている）

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// kunenbo::write_if_changed は経路を検査しないので再エクスポートしない（clippy.toml でも禁じている）
pub use kunenbo::{
    WriteOutcome, create_dir_all_under, ensure_no_symlink_under, ensure_symlink_free,
    list_files_under, remove_dir_all_under, remove_file_under, resolve_output_rel,
    write_atomic_under, write_under,
};

/// 前回書き出したが今回書き出さなかったファイルを削除し、
/// 空になったディレクトリを root 直前まで剪定する。削除件数を返す（[`kunenbo::remove_orphans`]）。
///
/// 前回マニフェストの不正なパス・経路にリンクがあるパスは消さずに飛ばし、大文字小文字を区別しない
/// FS で今回と同じファイルだった旧出力は消さずにパスの大文字小文字を付け直す（Phase 85）。
/// どちらも続けられないことがあれば警告のログを出す（文言とログの target は切り出しの前と同じ）
pub fn remove_orphans(
    root: &Path,
    previous: &BTreeSet<String>,
    current: &BTreeSet<String>,
) -> std::io::Result<usize> {
    kunenbo::remove_orphans(root, previous, current, |warning| match warning {
        kunenbo::OrphanWarning::Skipped { rel, error: e } => {
            let rel = rel.as_str();
            tracing::warn!(rel, "孤児掃除をスキップします: {e}");
        }
        // 付け直せなくても今回の出力は残っている（パスの大文字小文字が前回のままなだけ）
        kunenbo::OrphanWarning::CaseNotFixed { rel, error: e } => {
            let rel = rel.as_str();
            tracing::warn!(rel, "出力のパスの大文字小文字を付け直せません: {e}");
        }
        other => tracing::warn!("{other}"),
    })
}

/// 書き出したファイルの出どころ。書く側が申告する（記録を後からパスの文字列で分類しない =
/// 別の一覧を持つと v0.19.1 の R1 と同じずれが起きる）。build の要約と、利用者のファイルが
/// 生成物を上書きしたときの警告に使う（v0.20 Phase 91）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    /// ページの HTML（用語集・検索結果ページの合成ページを含む）
    Page,
    /// エイリアスのリダイレクト HTML
    Redirect,
    /// ページ単位の Markdown（原稿の `.md`）
    PageMd,
    /// サイト単位の生成物（404.html・llms・sitemap・テーマ資産・検索インデックス 等）
    Site(crate::SiteOutput),
    /// content の同伴アセット（利用者のファイル）
    ContentAsset,
    /// `public/` のコピー（利用者のファイル）
    Public,
}

impl OutputKind {
    /// setoka が作ったファイルか（利用者のファイルは同伴アセットと `public/`）
    pub fn is_generated(self) -> bool {
        !matches!(self, Self::ContentAsset | Self::Public)
    }
}

/// 利用者のファイルが、同じビルドで先に書いた setoka の生成物を上書きした記録
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overwrite {
    /// 上書きされた生成物の dist 相対パス（`/` 区切り。[`OutputRecord::files`] のキー）
    pub rel: String,
    /// 上書きされた生成物
    pub generated: OutputKind,
    /// 上書きした利用者のファイル（[`OutputKind::ContentAsset`] か [`OutputKind::Public`]）。
    /// 同伴アセットを `public/` がさらに上書きしたら、配信される `public/` の方
    pub by: OutputKind,
    /// 上書きした利用者のファイルの dist 相対パス。`rel` と違うのは、大文字小文字を区別しない
    /// FS で大文字小文字だけ違うパスが同じファイルになったとき（`public/INDEX.html` が
    /// `index.html` を上書きする）
    pub by_rel: String,
}

/// このビルドの出力の記録（[`OutputTracker::into_record`]）
#[derive(Debug, Default)]
pub struct OutputRecord {
    /// dist 相対パス → 最後に書いたものの出どころ
    pub files: std::collections::BTreeMap<String, OutputKind>,
    /// 利用者のファイルが生成物を上書きした記録（書いた順）
    pub overwrites: Vec<Overwrite>,
}

impl OutputRecord {
    /// 書いたパスの集合（孤児掃除とマニフェストの元）
    pub fn written(&self) -> BTreeSet<String> {
        self.files.keys().cloned().collect()
    }

    /// 出どころが `kind` のファイルの数
    pub fn count(&self, kind: OutputKind) -> usize {
        self.files.values().filter(|k| **k == kind).count()
    }
}

/// このビルドで書き出した dist 相対パスの記録（孤児掃除マニフェストの元）
pub struct OutputTracker {
    root: PathBuf,
    /// 2 つのパスが同じファイルか（テストで大文字小文字を区別しない FS を模すために差し替える）
    same: fn(&Path, &Path) -> bool,
    state: Mutex<TrackerState>,
}

#[derive(Default)]
struct TrackerState {
    record: OutputRecord,
    /// 記録したパスを、大文字小文字を畳んだキーで引く（同じファイルの候補探し）
    folded: std::collections::HashMap<String, Vec<String>>,
}

impl TrackerState {
    /// `rel`（書いたファイルは `path`）が、記録済みのどのパスと同じファイルか
    fn recorded_as(
        &self,
        root: &Path,
        rel: &str,
        path: &Path,
        same: fn(&Path, &Path) -> bool,
    ) -> Option<String> {
        if self.record.files.contains_key(rel) {
            return Some(rel.to_string());
        }
        self.folded
            .get(&rel.to_lowercase())?
            .iter()
            .find(|cur| resolve_output_rel(root, cur).is_ok_and(|cur_path| same(path, &cur_path)))
            .cloned()
    }
}

impl OutputTracker {
    /// 出力先そのものがシンボリックリンクなら作れない
    /// （書き込みが全部リンク先へ素通りするため。呼び出し側の検証と重なるが、
    /// tracker を作れば必ず書けてしまう以上ここでも止める二重防御）
    pub fn new(root: &Path) -> std::io::Result<Self> {
        Self::new_with(root, kunenbo::is_same_file)
    }

    fn new_with(root: &Path, same: fn(&Path, &Path) -> bool) -> std::io::Result<Self> {
        if root
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "出力ディレクトリ {} はシンボリックリンクです",
                    root.display()
                ),
            ));
        }
        Ok(Self {
            root: root.to_path_buf(),
            same,
            state: Mutex::new(TrackerState::default()),
        })
    }

    /// [`write_under`] ＋ 出力の記録。内容が同じで書き込みを省いたファイルも記録する
    /// （増分ビルドでも `--force` と同じ記録になる）。利用者のファイルが先に書いた生成物を
    /// 上書きしたら [`Overwrite`] に残す（`public/` が同伴アセットを上書きするのは仕様なので
    /// 残さない）。
    ///
    /// 記録は 1 ファイル 1 件。**大文字小文字を区別しない FS（macOS・Windows の既定）**では
    /// `public/INDEX.html` が生成した `index.html` と同じファイルに書かれるので、文字列が
    /// 違っても先に記録したパスの出どころを付け替え、上書きとして残す（PR #69 のレビュー指摘。
    /// 文字列だけで照合すると、利用者のファイルが配信されるのに警告が出ず、要約のページ数も
    /// 減らなかった）。キーは先に記録したパスのまま（ディスク上の名前も先に書いた方になる）。
    /// 判定は [`remove_orphans`] と同じくファイルの同一性で、大文字小文字を畳んだ文字列は
    /// 候補探しにだけ使う = 区別する FS では別のファイルとして記録する。Unicode の正規化
    /// （NFC / NFD）だけが違うパスは拾えない
    pub fn write(&self, kind: OutputKind, rel: &str, data: &[u8]) -> std::io::Result<WriteOutcome> {
        let (path, outcome) = write_under(&self.root, rel, data)?;
        let mut state = self.state.lock().unwrap();
        let Some(key) = state.recorded_as(&self.root, rel, &path, self.same) else {
            state.record.files.insert(rel.to_string(), kind);
            state
                .folded
                .entry(rel.to_lowercase())
                .or_default()
                .push(rel.to_string());
            return Ok(outcome);
        };
        let record = &mut state.record;
        let previous = record.files.insert(key.clone(), kind);
        match previous {
            Some(previous) if previous.is_generated() && !kind.is_generated() => {
                record.overwrites.push(Overwrite {
                    rel: key,
                    generated: previous,
                    by: kind,
                    by_rel: rel.to_string(),
                });
            }
            // 生成物を上書きした同伴アセットを、さらに `public/` が上書きした。配信されるのは
            // 後のファイルなので、上書きした側をそちらへ付け替える（PR #69 のレビュー指摘。
            // 警告が配信されない同伴アセットを挙げていた）
            Some(previous) if !previous.is_generated() && !kind.is_generated() => {
                if let Some(overwrite) = record.overwrites.iter_mut().find(|o| o.rel == key) {
                    overwrite.by = kind;
                    overwrite.by_rel = rel.to_string();
                }
            }
            _ => {}
        }
        Ok(outcome)
    }

    pub fn into_written(self) -> BTreeSet<String> {
        self.into_record().written()
    }

    pub fn into_record(self) -> OutputRecord {
        self.state.into_inner().unwrap().record
    }
}

/// 出力マニフェスト（前回書き出した dist 相対パス一覧）を読む。破損・不在は None
pub fn load_manifest(path: &Path) -> Option<BTreeSet<String>> {
    let bytes = fs::read(path).ok()?;
    let manifest: OutputManifest = serde_json::from_slice(&bytes).ok()?;
    (manifest.format_version == MANIFEST_FORMAT_VERSION).then_some(manifest.files)
}

/// マニフェストを書き出す。`path` の親（`.setoka/cache`）を基点に
/// ファイル名だけを [`write_under`] へ渡し、リンク経由の書き込みを防ぐ
pub fn save_manifest(path: &Path, files: &BTreeSet<String>) -> std::io::Result<()> {
    let invalid = |m: &str| std::io::Error::new(std::io::ErrorKind::InvalidInput, m.to_string());
    let parent = path
        .parent()
        .ok_or_else(|| invalid("親ディレクトリがありません"))?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| invalid("ファイル名を解決できません"))?;
    // 親ディレクトリ（基点）は write_under が作る（基点がリンクなら作らずに断る）

    let manifest = OutputManifest {
        format_version: MANIFEST_FORMAT_VERSION,
        files: files.clone(),
    };
    write_under(parent, name, &serde_json::to_vec(&manifest)?)?;
    Ok(())
}

const MANIFEST_FORMAT_VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutputManifest {
    format_version: u32,
    files: BTreeSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_orphans_は差分削除と空ディレクトリ剪定をする() {
        let dir = tempfile::tempdir().unwrap();
        let tracker = OutputTracker::new(dir.path()).unwrap();
        tracker.write(OutputKind::Page, "index.html", b"a").unwrap();
        tracker
            .write(OutputKind::Page, "old/index.html", b"b")
            .unwrap();
        tracker
            .write(OutputKind::Page, "keep/index.html", b"c")
            .unwrap();
        let previous = tracker.into_written();

        let tracker = OutputTracker::new(dir.path()).unwrap();
        tracker.write(OutputKind::Page, "index.html", b"a").unwrap();
        tracker
            .write(OutputKind::Page, "keep/other.html", b"d")
            .unwrap();
        let current = tracker.into_written();

        let removed = remove_orphans(dir.path(), &previous, &current).unwrap();
        assert_eq!(removed, 2);
        assert!(!dir.path().join("old").exists(), "空ディレクトリは剪定");
        assert!(dir.path().join("keep").exists(), "非空ディレクトリは残す");
        assert!(dir.path().join("index.html").exists());
    }

    #[test]
    fn output_tracker_は危険な相対パスを拒否する() {
        let dir = tempfile::tempdir().unwrap();
        let tracker = OutputTracker::new(dir.path()).unwrap();
        // `.` はファイルシステムが吸収するため、文字列比較では実ページと
        // 別物に見えるのに同じファイルを指す（エイリアス `"."` の再現）
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
            assert!(
                tracker.write(OutputKind::Page, bad, b"x").is_err(),
                "拒否されるべき: {bad:?}"
            );
        }
        assert!(tracker.write(OutputKind::Page, "a/b.html", b"x").is_ok());
        assert!(dir.path().join("a/b.html").exists());
    }

    /// 利用者のファイルが生成物を上書きしたら残す。`public/` が同伴アセットを上書きするのは
    /// 仕様なので残さない（v0.20 Phase 91）
    #[test]
    fn 利用者のファイルが生成物を上書きしたら記録する() {
        let dir = tempfile::tempdir().unwrap();
        let tracker = OutputTracker::new(dir.path()).unwrap();
        let llms = OutputKind::Site(crate::SiteOutput::LlmsTxt);
        tracker
            .write(OutputKind::Page, "index.html", b"page")
            .unwrap();
        tracker
            .write(OutputKind::PageMd, "index.md", b"md")
            .unwrap();
        tracker.write(llms, "llms.txt", b"llms").unwrap();
        tracker
            .write(OutputKind::ContentAsset, "img.png", b"a")
            .unwrap();
        tracker
            .write(OutputKind::ContentAsset, "index.html", b"asset")
            .unwrap();
        tracker.write(OutputKind::Public, "img.png", b"b").unwrap();
        tracker
            .write(OutputKind::Public, "llms.txt", b"public")
            .unwrap();
        let record = tracker.into_record();
        assert_eq!(
            record.overwrites,
            [
                Overwrite {
                    rel: "index.html".into(),
                    generated: OutputKind::Page,
                    by: OutputKind::ContentAsset,
                    by_rel: "index.html".into(),
                },
                Overwrite {
                    rel: "llms.txt".into(),
                    generated: llms,
                    by: OutputKind::Public,
                    by_rel: "llms.txt".into(),
                },
            ]
        );
        // 記録は最後に書いたものの出どころ
        assert_eq!(record.count(OutputKind::Page), 0);
        assert_eq!(record.count(OutputKind::PageMd), 1);
        assert_eq!(record.count(OutputKind::Public), 2);
        assert_eq!(record.written().len(), 4);
    }

    /// 同伴アセットが上書きした生成物を `public/` がさらに上書きしたら、配信される `public/` の
    /// ファイルを上書きした側として残す（PR #69 のレビュー指摘）
    #[test]
    fn 同伴アセットの上書きを_public_が上書きしたら上書きした側を付け替える() {
        let dir = tempfile::tempdir().unwrap();
        let case_insensitive = |a: &Path, b: &Path| {
            a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
        };
        let tracker = OutputTracker::new_with(dir.path(), case_insensitive).unwrap();
        tracker
            .write(OutputKind::Page, "index.html", b"page")
            .unwrap();
        tracker
            .write(OutputKind::ContentAsset, "index.html", b"asset")
            .unwrap();
        tracker
            .write(OutputKind::Page, "guide/index.html", b"page")
            .unwrap();
        tracker
            .write(OutputKind::ContentAsset, "guide/index.html", b"asset")
            .unwrap();
        // 大文字小文字だけ違う public/ のファイルが同じファイルに書かれる場合も同じ
        tracker
            .write(OutputKind::Public, "Guide/Index.html", b"public")
            .unwrap();
        tracker
            .write(OutputKind::Public, "index.html", b"public")
            .unwrap();
        let record = tracker.into_record();
        assert_eq!(
            record.overwrites,
            [
                Overwrite {
                    rel: "index.html".into(),
                    generated: OutputKind::Page,
                    by: OutputKind::Public,
                    by_rel: "index.html".into(),
                },
                Overwrite {
                    rel: "guide/index.html".into(),
                    generated: OutputKind::Page,
                    by: OutputKind::Public,
                    by_rel: "Guide/Index.html".into(),
                },
            ]
        );
        assert_eq!(record.count(OutputKind::Public), 2);
        assert_eq!(record.count(OutputKind::ContentAsset), 0);
    }

    /// 大文字小文字を区別しない FS（macOS・Windows の既定）: `public/INDEX.html` は生成した
    /// `index.html` と同じファイルに書かれる。文字列が違っても上書きとして残し、記録は
    /// 先に書いたパスの 1 件にまとめる（PR #69 のレビュー指摘）。Linux では同一性の判定を
    /// 差し替えて、その FS を模す
    #[test]
    fn 大文字小文字だけ違うパスでも同じファイルなら上書きとして記録する() {
        let dir = tempfile::tempdir().unwrap();
        let case_insensitive = |a: &Path, b: &Path| {
            a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
        };
        let tracker = OutputTracker::new_with(dir.path(), case_insensitive).unwrap();
        tracker
            .write(OutputKind::Page, "guide/index.html", b"page")
            .unwrap();
        tracker
            .write(OutputKind::ContentAsset, "Img.png", b"a")
            .unwrap();
        tracker
            .write(OutputKind::Public, "Guide/INDEX.html", b"user")
            .unwrap();
        tracker.write(OutputKind::Public, "img.PNG", b"b").unwrap();
        let record = tracker.into_record();
        assert_eq!(
            record.overwrites,
            [Overwrite {
                rel: "guide/index.html".into(),
                generated: OutputKind::Page,
                by: OutputKind::Public,
                by_rel: "Guide/INDEX.html".into(),
            }],
            "public/ が同伴アセットを上書きするのは仕様なので残さない"
        );
        assert_eq!(record.count(OutputKind::Page), 0);
        assert_eq!(record.count(OutputKind::ContentAsset), 0);
        assert_eq!(record.count(OutputKind::Public), 2);
        assert_eq!(
            record.written(),
            BTreeSet::from(["Img.png".to_string(), "guide/index.html".to_string()]),
            "記録は 1 ファイル 1 件で、キーは先に書いたパス"
        );
    }

    /// 大文字小文字を区別する FS（Linux の CI）: `index.html` と `INDEX.html` は別のファイル
    /// なので、上書きとして残さず 2 件を記録する（文字列を畳んだだけで同じとみなさない）
    #[test]
    fn 大文字小文字だけ違うパスは別のファイルなら別々に記録する() {
        let dir = tempfile::tempdir().unwrap();
        let tracker = OutputTracker::new(dir.path()).unwrap();
        tracker
            .write(OutputKind::Page, "index.html", b"page")
            .unwrap();
        tracker
            .write(OutputKind::Public, "INDEX.html", b"user")
            .unwrap();
        // 区別しない FS では同じファイルになるので、この前提が成り立つ環境だけで見る
        if fs::read(dir.path().join("index.html")).unwrap() == b"user" {
            return;
        }
        let record = tracker.into_record();
        assert_eq!(record.overwrites, []);
        assert_eq!(record.count(OutputKind::Page), 1);
        assert_eq!(record.count(OutputKind::Public), 1);
    }

    #[cfg(unix)]
    #[test]
    fn output_tracker_は出力先がリンクなら作れない() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        let link = dir.path().join("dist");
        std::os::unix::fs::symlink(&outside, &link).unwrap();

        assert!(OutputTracker::new(&link).is_err());
    }

    #[test]
    fn マニフェストのラウンドトリップと破損フォールバック() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        let files: BTreeSet<String> = ["a.html".to_string(), "b/c.html".to_string()].into();
        save_manifest(&path, &files).unwrap();
        assert_eq!(load_manifest(&path).unwrap(), files);

        fs::write(&path, b"{ broken").unwrap();
        assert!(load_manifest(&path).is_none());
        assert!(load_manifest(&dir.path().join("nothing.json")).is_none());
    }
}
