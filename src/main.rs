use gtk::prelude::*;
use gtk::{glib, Application};

mod crypto;
mod password_gen;
mod storage;
mod ui;
mod vault;

const APP_ID: &str = "org.exemplo.PasswordVault";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let window = crate::ui::build_main_window(app);
    window.present();
}