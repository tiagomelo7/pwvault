use std::cell::RefCell;
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;
use zeroize::Zeroize;

use crate::ui::list_screen::refresh_list_for_stack;
use crate::ui::state::AppState;

pub fn build_lock_screen(state: Rc<RefCell<AppState>>, stack: &gtk::Stack) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 12);

    let password_entry = gtk::PasswordEntry::new();
    password_entry.set_show_peek_icon(true);
    password_entry.set_placeholder_text(Some("Master PW"));

    let error_label = gtk::Label::new(Some(""));
    error_label.set_visible(false);
    error_label.set_xalign(0.0);
    error_label.add_css_class("error");

    let button = gtk::Button::with_label("desbloquear");

    container.append(&password_entry);
    container.append(&error_label);
    container.append(&button);

    let state_clone = Rc::clone(&state);
    let stack_clone = stack.clone();
    let password_entry_clone = password_entry.clone();
    let error_label_clone = error_label.clone();

    button.connect_clicked(move |_| {
        unlock_vault(
            Rc::clone(&state_clone),
            &stack_clone,
            &password_entry_clone,
            error_label_clone.clone(),
        );
    });

    container
}

fn unlock_vault(
    state: Rc<RefCell<AppState>>,
    stack: &gtk::Stack,
    password_entry: &gtk::PasswordEntry,
    error_label: gtk::Label,
) {
    let mut password = password_entry.text().to_string();
    if password.is_empty() {
        error_label.set_text("A senha não pode estar vazia.");
        error_label.set_visible(true);
        password.zeroize();
        password_entry.set_text("");
        return;
    }

    let state_clone = Rc::clone(&state);
    let stack_clone = stack.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let path = state.borrow().vault_path.clone();
    let password_for_thread = password.clone();

    std::thread::spawn(move || {
        let result = crate::vault::open_or_create(&path, &password_for_thread);
        let _ = sender.send(result);
    });

    password.zeroize();
    password_entry.set_text("");
    error_label.set_visible(false);

    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        match receiver.try_recv() {
            Ok(result) => match result {
                Ok(session) => {
                    state_clone.borrow_mut().session = Some(session);

                    if stack_clone.child_by_name("list").is_some() {
                        stack_clone.set_visible_child_name("list");
                        refresh_list_for_stack(Rc::clone(&state_clone), &stack_clone);
                    } else {
                        eprintln!("Tela 'list' não foi registrada no GtkStack");
                    }

                    glib::ControlFlow::Break
                }
                Err(error) => {
                    let message = match &error {
                        crate::vault::VaultError::WeakPassword(message) => message.clone(),
                        crate::vault::VaultError::WrongPassword => "Senha incorreta.".to_string(),
                        crate::vault::VaultError::EntryNotFound => "Entrada não encontrada.".to_string(),
                        crate::vault::VaultError::Io(_) => "Erro de acesso ao cofre.".to_string(),
                        crate::vault::VaultError::Serialization(_) => "Arquivo do cofre corrompido.".to_string(),
                    };
                    error_label.set_text(&message);
                    error_label.set_visible(true);
                    eprintln!("Erro ao abrir/criar cofre: {error:?}");
                    glib::ControlFlow::Break
                }
            },
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                eprintln!("A thread terminou sem retornar um resultado.");
                glib::ControlFlow::Break
            }
        }
    });
}

