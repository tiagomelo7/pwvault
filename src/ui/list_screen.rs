use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use zeroize::Zeroize;

use crate::ui::entry_form;
use crate::ui::state::AppState;

pub fn build_list_screen(state: Rc<RefCell<AppState>>, stack: &gtk::Stack) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 12);

    container.set_margin_top(12);
    container.set_margin_bottom(12);
    container.set_margin_start(12);
    container.set_margin_end(12);

    let search_entry = gtk::SearchEntry::new();
    search_entry.set_placeholder_text(Some("Buscar..."));
    let add_button = gtk::Button::with_label("Adicionar");
    let change_password_button = gtk::Button::with_label("Trocar senha mestra");

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_activate_on_single_click(true);

    container.append(&search_entry);
    container.append(&add_button);
    container.append(&change_password_button);
    container.append(&list);

    {
        let state = Rc::clone(&state);
        add_button.connect_clicked(move |_| {
            let _window = entry_form::build_entry_form(Rc::clone(&state), None);
            _window.present();
        });
    }

    {
        let state = Rc::clone(&state);
        let list = list.clone();

        search_entry.connect_search_changed(move |search_entry| {
            let query = search_entry.text();
            refresh_list(&list, Rc::clone(&state), query.as_str());
        });
    }

    {
        let state = Rc::clone(&state);
        let stack = stack.clone();
        change_password_button.connect_clicked(move |_| {
            let window = build_master_password_reset_dialog(Rc::clone(&state), &stack);
            window.present();
        });
    }

    container
}

fn build_master_password_reset_dialog(state: Rc<RefCell<AppState>>, stack: &gtk::Stack) -> gtk::Window {
    let window = gtk::Window::builder()
        .title("Trocar senha mestra")
        .default_width(420)
        .modal(true)
        .build();

    let container = gtk::Box::new(gtk::Orientation::Vertical, 12);
    container.set_margin_top(20);
    container.set_margin_bottom(20);
    container.set_margin_start(20);
    container.set_margin_end(20);

    let current_entry = gtk::PasswordEntry::new();
    current_entry.set_placeholder_text(Some("Senha atual"));
    current_entry.set_show_peek_icon(true);

    let new_entry = gtk::PasswordEntry::new();
    new_entry.set_placeholder_text(Some("Nova senha"));
    new_entry.set_show_peek_icon(true);

    let confirm_entry = gtk::PasswordEntry::new();
    confirm_entry.set_placeholder_text(Some("Confirmar nova senha"));
    confirm_entry.set_show_peek_icon(true);

    let info_label = gtk::Label::new(Some("Requer 12+ caracteres, maiúscula, minúscula, número e símbolo."));
    info_label.set_wrap(true);
    info_label.set_xalign(0.0);

    let error_label = gtk::Label::new(Some(""));
    error_label.set_visible(false);
    error_label.set_xalign(0.0);

    let apply_button = gtk::Button::with_label("Salvar nova senha");
    let cancel_button = gtk::Button::with_label("Cancelar");

    let form = gtk::Box::new(gtk::Orientation::Vertical, 8);
    form.append(&current_entry);
    form.append(&new_entry);
    form.append(&confirm_entry);
    form.append(&info_label);
    form.append(&error_label);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.append(&cancel_button);
    buttons.append(&apply_button);

    container.append(&form);
    container.append(&buttons);
    window.set_child(Some(&container));

    {
        let window = window.clone();
        cancel_button.connect_clicked(move |_| {
            window.close();
        });
    }

    {
        let state = Rc::clone(&state);
        let stack = stack.clone();
        let window = window.clone();

        apply_button.connect_clicked(move |_| {
            let mut current = current_entry.text().to_string();
            let mut new_password = new_entry.text().to_string();
            let mut confirm = confirm_entry.text().to_string();

            if current.is_empty() || new_password.is_empty() || confirm.is_empty() {
                error_label.set_text("Preencha todos os campos.");
                error_label.set_visible(true);
                current.zeroize();
                new_password.zeroize();
                confirm.zeroize();
                return;
            }

            if new_password != confirm {
                error_label.set_text("As senhas não conferem.");
                error_label.set_visible(true);
                current.zeroize();
                new_password.zeroize();
                confirm.zeroize();
                return;
            }

            match crate::vault::validate_master_password(&new_password) {
                Ok(()) => {}
                Err(error) => {
                    let message = match error {
                        crate::vault::VaultError::WeakPassword(message) => message,
                        _ => "Senha inválida.".to_string(),
                    };
                    error_label.set_text(&message);
                    error_label.set_visible(true);
                    current.zeroize();
                    new_password.zeroize();
                    confirm.zeroize();
                    return;
                }
            }

            let path = state.borrow().vault_path.clone();
            match crate::vault::change_master_password(&path, &current, &new_password) {
                Ok(()) => {
                    let mut state_ref = state.borrow_mut();
                    state_ref.session = None;
                    drop(state_ref);
                    stack.set_visible_child_name("lock");
                    window.close();
                }
                Err(error) => {
                    let message = match error {
                        crate::vault::VaultError::WrongPassword => "Senha atual incorreta.".to_string(),
                        crate::vault::VaultError::WeakPassword(message) => message,
                        _ => "Não foi possível trocar a senha mestra.".to_string(),
                    };
                    error_label.set_text(&message);
                    error_label.set_visible(true);
                }
            }

            current.zeroize();
            new_password.zeroize();
            confirm.zeroize();
        });
    }

    window
}

pub fn refresh_list_for_stack(state: Rc<RefCell<AppState>>, stack: &gtk::Stack) {
    let Some(child) = stack.child_by_name("list") else {
        return;
    };

    let Ok(screen) = child.downcast::<gtk::Box>() else {
        return;
    };

    let Some(list) = screen.last_child().and_then(|child| child.downcast::<gtk::ListBox>().ok()) else {
        return;
    };

    refresh_list(&list, state, "");
}

fn refresh_list(list: &gtk::ListBox, state: Rc<RefCell<AppState>>, query: &str) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let state_rc = Rc::clone(&state);
    let state_ref = state_rc.borrow();
    let Some(session) = &state_ref.session else {
        return;
    };

    for entry in filter_entries(session, query) {
        let row = gtk::ListBoxRow::new();
        row.set_activatable(true);

        let label = gtk::Label::new(Some(entry.site.as_str()));
        label.set_xalign(0.0);
        row.set_child(Some(&label));

        let state_for_row = Rc::clone(&state);
        let entry_for_row = entry.clone();

        let state_for_click = Rc::clone(&state_for_row);
        let entry_for_click = entry_for_row.clone();

        let click = gtk::GestureClick::new();
        click.set_button(0);
        click.connect_pressed(move |_, _, _, _| {
            let window = entry_form::build_entry_form(Rc::clone(&state_for_click), Some(entry_for_click.clone()));
            window.present();
        });
        row.add_controller(click);

        let state_for_activate = Rc::clone(&state_for_row);
        let entry_for_activate = entry_for_row.clone();
        row.connect_activate(move |_| {
            let window = entry_form::build_entry_form(Rc::clone(&state_for_activate), Some(entry_for_activate.clone()));
            window.present();
        });

        list.append(&row);
    }
}

fn filter_entries<'a>(session: &'a crate::vault::VaultSession, query: &str) -> Vec<&'a crate::vault::Entry> {
    let query = query.to_lowercase();
    session
        .entries
        .iter()
        .filter(|entry| {
            entry.site.to_lowercase().contains(&query)
                || entry.username.to_lowercase().contains(&query)
                || entry.notes
                    .as_deref()
                    .map(|notes| notes.to_lowercase().contains(&query))
                    .unwrap_or(false)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::ui::state::AppState;
    use crate::vault::{Entry, VaultSession};
    use uuid::Uuid;

    #[test]
    fn refresh_list_for_stack_presents_entries_without_search_query() {
        let state = Rc::new(RefCell::new(AppState {
            session: Some(VaultSession {
                key: [0u8; 32],
                salt: vec![],
                entries: vec![Entry {
                    id: Uuid::new_v4(),
                    site: "github.com".to_string(),
                    username: "alice".to_string(),
                    password: "secret".to_string(),
                    notes: None,
                }],
            }),
            vault_path: std::env::temp_dir().join("pwvault-test-vault"),
        }));

        let stack = gtk::Stack::new();
        let screen = build_list_screen(Rc::clone(&state), &stack);
        stack.add_named(&screen, Some("list"));
        stack.set_visible_child_name("list");

        let list_screen = stack.child_by_name("list").unwrap();
        let box_widget = list_screen.downcast::<gtk::Box>().unwrap();
        let list = box_widget
            .last_child()
            .and_then(|child| child.downcast::<gtk::ListBox>().ok())
            .expect("a lista deve existir no stack");

        assert!(list.first_child().is_some(), "a lista deve mostrar as entradas do cofre");
    }
}