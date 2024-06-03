use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Manager, Runtime};

use crate::error::{Error, Result};

use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Mutex,
    fs::{self},
};

use fantoccini::{wd::TimeoutConfiguration, Client, ClientBuilder};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Access to the fanto APIs.
#[allow(dead_code)]
pub struct Fanto<R: Runtime> {
    app: AppHandle<R>,
    app_local_data_dir: PathBuf,
    driver_path: PathBuf,
    process: Mutex<Child>,
    port: u16,
}

impl<R: Runtime> Fanto<R> {
    pub fn init<C: DeserializeOwned>(
        app: &AppHandle<R>,
        _api: PluginApi<R, C>,
    ) -> crate::Result<Fanto<R>> {
        let app_local_data_dir = app.path().app_local_data_dir()?;
        if !app_local_data_dir.is_dir() {
            std::fs::create_dir(&app_local_data_dir)?;
        }

        let driver_path =
            tauri::async_runtime::block_on(async { dowload_webdriver(&app_local_data_dir).await })?;

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
            app_local_data_dir,
            driver_path,
            process: Mutex::new(process),
            port,
        })
    }

    pub fn destroy(&self) {
        let mut process = self.process.lock().unwrap();
        let _ = process.kill();
    }

    pub async fn driver(&self) -> Result<Client> {
        #[cfg(target_os = "macos")]
        let driver = chrome_client(self.port, &self.app_local_data_dir).await?;
        #[cfg(target_os = "windows")]
        let driver = edge_client(self.port, &self.app_local_data_dir).await?;

        let _ = driver
            .set_ua("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .await;
        let _ = driver
            .update_timeouts(TimeoutConfiguration::new(
                Some(std::time::Duration::from_secs(60)),
                Some(std::time::Duration::from_secs(60)),
                Some(std::time::Duration::from_secs(15)),
            ))
            .await;
        Ok(driver)
    }
}

async fn dowload_webdriver(tauri_dir: &PathBuf) -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    let driver_path = tauri_dir.join("chromedriver");
    #[cfg(target_os = "windows")]
    let driver_path = tauri_dir.join("msedgedriver.exe");
    if !driver_path.is_file() {
        #[cfg(target_os = "macos")]
        dowload_chromedriver(&driver_path).await?;
        #[cfg(target_os = "windows")]
        dowload_msedgedriver(&driver_path).await?;
    }

    #[cfg(target_os = "macos")]
    todo!();
    #[cfg(target_os = "windows")]
    let (driver_version, browser_version) = {(
        msedgedriver_version(&driver_path)?,
        msedge_version()?,
    )};

    if driver_version != browser_version {
        #[cfg(target_os = "macos")]
        dowload_chromedriver(&driver_path).await?;
        #[cfg(target_os = "windows")]
        dowload_msedgedriver(&driver_path).await?;
    }

    Ok(driver_path)
}

#[cfg(target_os = "macos")]
async fn dowload_chromedriver(driver_path: &PathBuf) -> Result<()> {
    use webdriver_downloader::driver_impls::chromedriver_for_testing_info::ChromedriverForTestingInfo;
    use webdriver_downloader::prelude::*;

    let mut old_driver_info = ChromedriverOldInfo::new_default()?;
    let mut driver_info = ChromedriverForTestingInfo::new_default()?;
    driver_info.browser_path = old_driver_info.browser_path;

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
    let msedge_version = msedge_version()?;
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
fn msedge_version() -> Result<String> {
    let edge_executable = PathBuf::from("C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe");
    check_version(&edge_executable)
}

#[cfg(target_os = "windows")]
fn msedgedriver_version(driver_path: &PathBuf) -> Result<String> {
    check_version(driver_path)
}

#[cfg(target_os = "windows")]
fn check_version(executable: &PathBuf) -> Result<String> {
    if fs::metadata(executable).is_ok() {
        let output = Command::new("powershell")
            .arg("-Command")
            .arg(format!(
                "(Get-Item '{}').VersionInfo.ProductVersion",
                executable.to_string_lossy()
            ))
            .output()?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            Ok(version.trim().to_string())
        } else {
            Err(Error::VersionNotFound(String::from_utf8(output.stderr)?))
        }
    } else {
        Err(Error::ExecutableNotFound(executable.to_owned()))
    }
}

#[cfg(target_os = "macos")]
async fn chrome_client(port: u16, tauri_path: &PathBuf) -> Result<Client> {
    Ok(ClientBuilder::native()
        .capabilities(
            [(
                String::from("goog:chromeOptions"),
                serde_json::json!({
                    "args": [
                        // "--headless",
                        "--incognito",
                        &format!("--user-data-dir={}/driver-user-data", tauri_path.display()),
                    ],
                }),
            )]
            .into_iter()
            .collect(),
        )
        .connect(&format!("http://localhost:{}", port))
        .await?)
}

#[cfg(target_os = "windows")]
async fn edge_client(port: u16, tauri_path: &PathBuf) -> Result<Client> {
    Ok(ClientBuilder::native()
        .capabilities(
            [(
                String::from("ms:edgeOptions"),
                serde_json::json!({
                    "args": [
                        // "--headless",
                        "-inprivate",
                        &format!("--user-data-dir={}\\driver-user-data", tauri_path.display()),
                    ],
                }),
            )]
            .into_iter()
            .collect(),
        )
        .connect(&format!("http://localhost:{}", port))
        .await?)
}
