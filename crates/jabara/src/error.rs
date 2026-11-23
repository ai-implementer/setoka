/// 配信・監視のエラー（種類は後から増えることがある）
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServerError {
    #[error("サーバの起動に失敗しました: {0}")]
    Io(#[from] std::io::Error),

    /// ポートが使用中。`{0}` をそのまま出すと「Address already in use (os error 48)」
    /// だけになり、**どのポートか**が分からない。回避の案内（別のポートの指定の仕方）は
    /// 使う側のコマンドで違うので、使う側が足す（setoka は `--port` を案内する）
    #[error("ポート {port} は既に使用中です（{host} にバインドできません）")]
    PortInUse { host: std::net::IpAddr, port: u16 },

    #[error("ファイル監視に失敗しました: {0}")]
    Notify(#[from] notify::Error),

    /// `ServeOptions::live_reload_path` が使えない形（`/` で始まらない等）
    #[error("{0}")]
    InvalidLiveReloadPath(String),

    /// 監視中の再ビルドが panic して監視スレッドが止まったので、配信も止めた
    #[error("監視中の再ビルドで内部エラーが起きたため、配信を終了しました: {0}")]
    WatchStopped(String),
}
