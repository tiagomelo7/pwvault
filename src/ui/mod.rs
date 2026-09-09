use std::cell::RefCell;
use std::rc::Rc;
use gtk::glib;
use gtk::prelude::*;

pub mod clipboard;
pub mod entry_form;
pub mod list_screen;
pub mod lock_screen;
pub mod state;

use crate::storage;
use crate::ui::state::AppState;

fn clear_session_data(state: &Rc<RefCell<AppState>>) {
    let mut state_ref = state.borrow_mut();
    if let Some(mut session) = state_ref.session.take() {
        session.clear_sensitive_data();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_stack_contains_list_child() {
        let app = gtk::Application::builder()
            .application_id("pwvault.tests")
            .build();

        let window = build_main_window(&app);
        let stack = window
            .child()
            .and_then(|widget| widget.downcast::<gtk::Stack>().ok())
            .expect("a janela deveria conter um gtk::Stack");

        assert!(stack.child_by_name("list").is_some(), "a tela de lista deve existir no stack");
    }
}

pub fn build_main_window(app: &gtk::Application) -> gtk::ApplicationWindow {
    let vault_path = storage::default_vault_path();
    let state = Rc::new(RefCell::new(AppState {
        session: None,
        vault_path,
    }));
    let stack = gtk::Stack::new();
    let lock_screen = lock_screen::build_lock_screen(Rc::clone(&state), &stack);
    let list_screen = list_screen::build_list_screen(Rc::clone(&state), &stack);

    stack.add_named(&lock_screen, Some("lock"));
    stack.add_named(&list_screen, Some("list"));
    stack.set_visible_child_name("lock");

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Password Vault")
        .default_width(600)
        .default_height(500)
        .child(&stack)
        .build();

    setup_inactivity_timer(Rc::clone(&state), &window, 300);

    window
}

pub fn setup_inactivity_timer(
    state: Rc<RefCell<AppState>>,
    window: &gtk::ApplicationWindow,
    timeout_secs: u32,
) {
    let stack = window
        .child()
        .and_then(|widget| widget.downcast::<gtk::Stack>().ok())
        .expect("A janela deveria conter um gtk::Stack");

    let timer_id: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    let reset_timer = {
        let timer_id = Rc::clone(&timer_id);
        let state = Rc::clone(&state);
        let stack = stack.clone();

        Rc::new(move || {
            if let Some(source_id) = timer_id.borrow_mut().take() {
                source_id.remove();
            }

            let state = Rc::clone(&state);
            let stack = stack.clone();
            let timer_id_for_closure = Rc::clone(&timer_id);
            let source_id = glib::timeout_add_seconds_local(timeout_secs, move || {
                clear_session_data(&state);
                stack.set_visible_child_name("lock");
                timer_id_for_closure.borrow_mut().take();

                glib::ControlFlow::Break
            });

            *timer_id.borrow_mut() = Some(source_id);
        })
    };
    reset_timer();

    let motion_controller = gtk::EventControllerMotion::new();
    {
        let reset_timer = Rc::clone(&reset_timer);
        motion_controller.connect_motion(move |_, _, _| {
            reset_timer();
        });
    }
    window.add_controller(motion_controller);

    let key_controller = gtk::EventControllerKey::new();
    {
        let reset_timer = Rc::clone(&reset_timer);
        key_controller.connect_key_pressed(move |_, _, _, _| {
            reset_timer();
            glib::Propagation::Proceed
        });
    }
    window.add_controller(key_controller);
}