use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error(transparent)]
  Io(#[from] std::io::Error),
  #[cfg(mobile)]
  #[error(transparent)]
  PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),

  #[error(transparent)]
  Tauri(#[from] tauri::Error),

  #[cfg(target_os = "macos")]
  #[error(transparent)]
  WebdriverDownloadError(#[from] webdriver_downloader::prelude::WebdriverDownloadError),
  #[cfg(target_os = "macos")]
  #[error(transparent)]
  DefaultPathError(#[from] webdriver_downloader::os_specific::DefaultPathError),
  #[cfg(target_os = "macos")]
  #[error("Browser is not installed in `{0}`")]
  BrowserNotFound(std::path::PathBuf),

  #[cfg(target_os = "windows")]
  #[error("MsEdge version not found")]
  MsEdgeVersionNotFound,
  #[cfg(target_os = "windows")]
  #[error(transparent)]
  ReqwestError(#[from] reqwest::Error),
  #[cfg(target_os = "windows")]
  #[error(transparent)]
  ZipError(#[from] zip::result::ZipError),
}

impl Serialize for Error {
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(self.to_string().as_ref())
  }
}
