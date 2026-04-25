//! Vendored `svls` language-server adapter for `vuff`.
//!
//! The implementation is copied from `svls` and rewired so `vuff` remains the
//! public frontend and configuration is loaded from `vuff.toml`.

mod backend;
pub mod config;

use backend::Backend;
use simplelog::{Config as LogConfig, LevelFilter, WriteLogger};
use std::fs::File;
use std::path::{Path, PathBuf};
use tower_lsp::{LspService, Server};

pub const CONFIG_FILE_NAME: &str = vuff_config::CONFIG_FILE_NAME;

#[must_use]
pub fn find_config_file(start: &Path) -> Option<PathBuf> {
    vuff_config::find_config_file(start)
}

pub async fn serve_stdio(debug: bool) -> std::io::Result<()> {
    if debug {
        let _ = WriteLogger::init(
            LevelFilter::Debug,
            LogConfig::default(),
            File::create("vuff-server.log")?,
        );
    }

    log::debug!("start");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, messages) = LspService::new(Backend::new);
    Server::new(stdin, stdout, messages).serve(service).await;
    Ok(())
}

pub fn run_stdio(debug: bool) -> std::io::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(serve_stdio(debug))
}
