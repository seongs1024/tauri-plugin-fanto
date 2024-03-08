use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Manager, Runtime};

use crate::models::*;

use crate::error::{Error, Result};

use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Mutex,
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Access to the fanto APIs.
#[allow(dead_code)]
pub struct Fanto<R: Runtime> {
    app: AppHandle<R>,
    driver_path: PathBuf,
    process: Mutex<Child>,
    port: u16,
}

impl<R: Runtime> Fanto<R> {
    pub fn ping(&self, payload: PingRequest) -> crate::Result<PingResponse> {
        Ok(PingResponse {
            value: payload.value,
        })
    }

    pub fn init<C: DeserializeOwned>(
        app: &AppHandle<R>,
        _api: PluginApi<R, C>,
    ) -> crate::Result<Fanto<R>> {
        let app_clone = app.clone();
        let driver_path =
            tauri::async_runtime::block_on(async move { dowload_webdriver(&app_clone).await })?;

        let mut port = 4444;
        let process = loop {
            match std::net::TcpListener::bind(("localhost", port)) {
                Ok(_) => {}
                Err(_) => {
                    port += 1;
                    continue;
                }
            };

            #[cfg(not(target_os = "windows"))]
            let mut process = Command::new(&driver_path)
                .args([format!("--port={}", port)])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;

            //const CREATE_NO_WINDOW: u32 = 0x08000000;
            #[cfg(target_os = "windows")]
            let mut process = Command::new(&driver_path)
                .args([format!("--port={}", port)])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(0x08000000)
                .spawn()?;

            println!("webdriver process's ID is {}", process.id());
            let status = process.try_wait()?;
            if status.is_none() {
                break process;
            }
            port += 1;
        };

        Ok(Fanto {
            app: app.clone(),
            driver_path,
            process: Mutex::new(process),
            port,
        })
    }

    pub fn destroy(&self) {
        let mut process = self.process.lock().unwrap();
        let _ = process.kill();
    }
}

async fn dowload_webdriver<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf> {
    let tauri_dir = app.path().app_local_data_dir()?;
    if !tauri_dir.is_dir() {
        std::fs::create_dir(&tauri_dir)?;
    }

    #[cfg(target_os = "macos")]
    let driver_path = tauri_dir.join("chromedriver");
    #[cfg(target_os = "windows")]
    let driver_path = tauri_dir.join("msedgedriver.exe");
    if driver_path.is_file() {
        return Ok(driver_path);
    }

    #[cfg(target_os = "macos")]
    dowload_chromedriver(&driver_path).await?;
    #[cfg(target_os = "windows")]
    dowload_msedgedriver(&driver_path).await?;

    Ok(driver_path)
}

#[cfg(target_os = "macos")]
async fn dowload_chromedriver(driver_path: &PathBuf) -> Result<()> {
    use webdriver_downloader::prelude::*;

    let mut driver_info = ChromedriverOldInfo::new_default()?;

    if !driver_info.browser_path.is_file() {
        return Err(Error::BrowserNotFound(driver_info.browser_path));
    }

    driver_info.driver_install_path = driver_path.to_path_buf();

    if !driver_info.is_installed().await {
        driver_info.download_install().await?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
async fn dowload_msedgedriver(driver_path: &PathBuf) -> Result<()> {
    let msedge_version = msedgedriver_version()?;
    let url = format!(
        "https://msedgedriver.azureedge.net/{}/edgedriver_win64.zip",
        msedge_version
    );
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko)",
        )
        .build()?;
    let res = client.get(&url).send().await?;

    let bytes = res.bytes().await?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if file.name() == "msedgedriver.exe" {
            let mut f = std::fs::File::create(driver_path)?;
            std::io::copy(&mut file, &mut f)?;
            break;
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn msedgedriver_version() -> Result<String> {
    std::fs::read_dir("C:\\Program Files (x86)\\Microsoft\\Edge\\Application")?
        .flat_map(|entry| entry)
        .filter(|entry| match entry.file_type() {
            Ok(file_type) => file_type.is_dir(),
            Err(_) => false,
        })
        .filter_map(|entry| match entry.path().file_name() {
            Some(file_name) => match file_name.to_str() {
                Some(file_name) => Some(file_name.to_string()),
                None => None,
            },
            None => None,
        })
        .filter(|file_name| file_name.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .take(1)
        .next()
        .ok_or(Error::MsEdgeVersionNotFound)
}
