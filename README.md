# Tauri Plugin fanto

[Fantoccini](https://github.com/jonhoo/fantoccini) integrated with [webdriver downloader](https://github.com/ik1ne/webdriver-downloader)


## Install

`src-tauri/Cargo.toml`

```toml
[dependencies]
tauri-plugin-fanto = "0.2.0"
```

## Usage

`src-tauri/src/main.rs`

```rust
use tauri::AppHandle;
use tauri_plugin_fanto::{
    FantoExt,
    fantoccini::{
        Locator,
        key::Key,
    },
    Error,
};

#[tauri::command]
async fn greet(app: tauri::AppHandle) -> Result<(), Error> {
    let fanto = app.fanto();

    let driver = fanto.driver().await?;

    driver.goto("https://www.example.com").await?;
    driver.find(Locator::XPath("//a")).await?
        .click().await?;

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fanto::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```