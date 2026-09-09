use std::time::Duration;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

pub fn copy_to_clipboard_with_timeout(display: &gdk::Display, text: &str, seconds: u32) {
    let clipboard = display.clipboard();
    let copied_text = text.to_string();
    clipboard.set_text(&copied_text);

    let clipboard_clone = clipboard.clone();
    glib::timeout_add_local(Duration::from_secs(seconds as u64), move || {
        let clipboard_clone = clipboard_clone.clone();
        let expected_text = copied_text.clone();

        glib::MainContext::default().spawn_local(async move {
            if let Ok(current_text) = clipboard_clone.read_text_future().await {
                if current_text.as_deref() == Some(expected_text.as_str()) {
                    clipboard_clone.set_text("");
                }
            }
        });

        glib::ControlFlow::Break
    });
}