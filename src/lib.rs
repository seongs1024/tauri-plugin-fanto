use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, RunEvent, Runtime,
};

use std::{collections::HashMap, sync::Mutex};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Fanto;
#[cfg(mobile)]
use mobile::Fanto;

#[derive(Default)]
struct MyState(Mutex<HashMap<String, String>>);

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the fanto APIs.
pub trait FantoExt<R: Runtime> {
    fn fanto(&self) -> &Fanto<R>;
}

impl<R: Runtime, T: Manager<R>> crate::FantoExt<R> for T {
    fn fanto(&self) -> &Fanto<R> {
        self.state::<Fanto<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("fanto")
        .invoke_handler(tauri::generate_handler![commands::execute])
        .setup(|app, api| {
            #[cfg(mobile)]
            let fanto = mobile::init(app, api)?;
            #[cfg(desktop)]
            let fanto = desktop::Fanto::init(app, api)?;
            app.manage(fanto);

            app.manage(MyState::default());
            Ok(())
        })
        .on_event(|app, event| {
            if let RunEvent::Exit = event {
                if let Some(fanto) = app.try_state::<Fanto<R>>() {
                    #[cfg(desktop)]
                    fanto.destroy();
                };
            }
        })
        .build()
}
