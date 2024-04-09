use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, RunEvent, Runtime,
};

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod error;

pub use error::{Error, Result};

pub use fantoccini;

#[cfg(desktop)]
use desktop::Fanto;
#[cfg(mobile)]
use mobile::Fanto;

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
        .setup(|app, api| {
            #[cfg(mobile)]
            let fanto = mobile::init(app, api)?;
            #[cfg(desktop)]
            let fanto = desktop::Fanto::init(app, api)?;
            app.manage(fanto);
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
