use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;

use crate::password_gen;
use crate::storage;
use crate::ui::clipboard;
use crate::ui::state::AppState;
use crate::vault::Entry;

pub fn build_entry_form(state: Rc<RefCell<AppState>>, existing: Option<Entry>) -> gtk::Window {
    let window = gtk::Window::builder()
        .title(if existing.is_some() {
            "Editar entrada"
        } else {
            "Nova entrada"
        })
        .default_width(500)
        .default_height(500)
        .modal(true)
        .build();

    let container = gtk::Box::new(gtk::Orientation::Vertical, 12);

    container.set_margin_top(20);
    container.set_margin_bottom(20);
    container.set_margin_start(20);
    container.set_margin_end(20);

    let site_entry = gtk::Entry::new();
    site_entry.set_placeholder_text(Some("Site"));

    let username_entry = gtk::Entry::new();
    username_entry.set_placeholder_text(Some("Usuário"));

    let password_entry = gtk::PasswordEntry::new();
    password_entry.set_placeholder_text(Some("Senha"));
    password_entry.set_show_peek_icon(true);

    let notes_entry = gtk::TextView::new();
    notes_entry.set_vexpand(true);

    let generate_button = gtk::Button::with_label("Gerar senha");
    let copy_button = gtk::Button::with_label("Copiar");
    let delete_button = gtk::Button::with_label("Excluir");
    let save_button = gtk::Button::with_label("Salvar");

    let password_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);

    password_box.append(&password_entry);
    password_box.append(&generate_button);
    password_box.append(&copy_button);

    container.append(&site_entry);
    container.append(&username_entry);
    container.append(&password_box);
    container.append(&notes_entry);
    if existing.is_some() {
        container.append(&delete_button);
    }
    container.append(&save_button);

    if let Some(entry) = &existing {
        site_entry.set_text(&entry.site);
        username_entry.set_text(&entry.username);
        password_entry.set_text(&entry.password);

        let buffer = notes_entry.buffer();
        buffer.set_text(entry.notes.as_deref().unwrap_or(""));
    }

    {
        let password_entry = password_entry.clone();

        generate_button.connect_clicked(move |_| {
            let options = password_gen::GeneratorOptions {
                length: 32,
                use_uppercase: true,
                use_lowercase: true,
                use_digits: true,
                use_symbols: true,
            };

            let password = password_gen::generate_password(&options);
            password_entry.set_text(&password);
        });
    }

    {
        let password_entry = password_entry.clone();

        copy_button.connect_clicked(move |_| {
            let display = password_entry.display();
            let password = password_entry.text();
            clipboard::copy_to_clipboard_with_timeout(&display, password.as_str(), 30);
        });
    }

    {
        let state = Rc::clone(&state);
        let window = window.clone();

        let site_entry = site_entry.clone();
        let username_entry = username_entry.clone();
        let password_entry = password_entry.clone();
        let notes_entry = notes_entry.clone();
        let existing_for_save = existing.clone();

        save_button.connect_clicked(move |_| {
            let site = site_entry.text().to_string();
            let username = username_entry.text().to_string();
            let password = password_entry.text().to_string();

            let notes_buffer = notes_entry.buffer();
            let notes = notes_buffer
                .text(
                    &notes_buffer.start_iter(),
                    &notes_buffer.end_iter(),
                    false,
                )
                .to_string();

            let mut state = state.borrow_mut();

            let Some(session) = state.session.as_mut() else {
                return;
            };

            if let Some(existing_entry) = &existing_for_save {
                let updated_entry = Entry {
                    id: existing_entry.id,
                    site,
                    username,
                    password,
                    notes: Some(notes),
                };

                if let Err(error) = crate::vault::update_entry(session, updated_entry) {
                    eprintln!("Erro ao atualizar entrada: {error:?}");
                    return;
                }
            } else {
                let new_entry = Entry {
                    id: uuid::Uuid::new_v4(),
                    site,
                    username,
                    password,
                    notes: Some(notes),
                };

                crate::vault::add_entry(session, new_entry);
            }

            let bytes = match crate::vault::save_vault(session) {
                Ok(bytes) => bytes,
                Err(error) => {
                    eprintln!("Erro ao salvar cofre: {error:?}");
                    return;
                }
            };

            if let Err(error) = storage::write_vault_file(&state.vault_path, &bytes) {
                eprintln!("Erro ao escrever arquivo: {error:?}");
                return;
            }

            drop(state);
            window.close();
        });
    }

    if existing.is_some() {
        let state = Rc::clone(&state);
        let window = window.clone();
        let existing_for_delete = existing.clone();

        delete_button.connect_clicked(move |_| {
            let mut state = state.borrow_mut();
            let Some(session) = state.session.as_mut() else {
                return;
            };

            let Some(entry_to_delete) = &existing_for_delete else {
                return;
            };

            if let Err(error) = crate::vault::remove_entry(session, entry_to_delete.id) {
                eprintln!("Erro ao excluir entrada: {error:?}");
                return;
            }

            let bytes = match crate::vault::save_vault(session) {
                Ok(bytes) => bytes,
                Err(error) => {
                    eprintln!("Erro ao salvar cofre após exclusão: {error:?}");
                    return;
                }
            };

            if let Err(error) = storage::write_vault_file(&state.vault_path, &bytes) {
                eprintln!("Erro ao escrever arquivo após exclusão: {error:?}");
                return;
            }

            drop(state);
            window.close();
        });
    }

    window.set_child(Some(&container));
    window
}