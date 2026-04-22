use super::*;
use std::{cell::RefCell, rc::Rc};

#[test]
fn pick_file_path_sequence_restores_terminal_after_selection() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let shared = events.clone();
    let result = pick_file_path_with_ops(
        |keyboard_enhancement_enabled| {
            shared
                .borrow_mut()
                .push(format!("leave:{keyboard_enhancement_enabled}"));
            Ok(())
        },
        || {
            shared.borrow_mut().push("pick".to_string());
            Ok(Some(PathBuf::from("/tmp/doc.md")))
        },
        || {
            shared.borrow_mut().push("enter:true".to_string());
            Ok(true)
        },
        || {
            shared.borrow_mut().push("clear".to_string());
            Ok(())
        },
    )
    .expect("picker should succeed");

    assert_eq!(result, Some(PathBuf::from("/tmp/doc.md")));
    assert_eq!(
        events.borrow().as_slice(),
        ["leave:false", "pick", "enter:true", "clear"]
    );
}

#[test]
fn pick_file_path_treats_enter_failure_as_fatal_restore_error() {
    let error = pick_file_path_with_ops(
        |_| Ok(()),
        || Ok(None),
        || Err("enter failed".to_string()),
        || Ok(()),
    )
    .expect_err("enter failure should be fatal");

    assert_eq!(
        error,
        PickFilePathError::TerminalState("enter failed".to_string())
    );
}

#[test]
fn pick_file_path_treats_clear_failure_as_fatal_restore_error() {
    let error = pick_file_path_with_ops(
        |_| Ok(()),
        || Ok(None),
        || Ok(false),
        || Err("clear failed".to_string()),
    )
    .expect_err("clear failure should be fatal");

    assert_eq!(
        error,
        PickFilePathError::TerminalState("clear failed".to_string())
    );
}

#[test]
fn pick_file_path_treats_leave_failure_as_fatal_terminal_error() {
    let error = pick_file_path_with_ops(
        |_| Err("leave failed".to_string()),
        || Ok(None),
        || Ok(false),
        || Ok(()),
    )
    .expect_err("leave failure should be fatal");

    assert_eq!(
        error,
        PickFilePathError::TerminalState("leave failed".to_string())
    );
}

#[test]
fn pick_file_path_returns_picker_error_only_after_successful_restore() {
    let error = pick_file_path_with_ops(
        |_| Ok(()),
        || Err("dialog failed".to_string()),
        || Ok(false),
        || Ok(()),
    )
    .expect_err("picker failure should return picker error after restore");

    assert_eq!(
        error,
        PickFilePathError::Picker("dialog failed".to_string())
    );
}

#[test]
fn pick_file_path_prioritizes_restore_failure_after_picker_error() {
    let error = pick_file_path_with_ops(
        |_| Ok(()),
        || Err("dialog failed".to_string()),
        || Err("enter failed".to_string()),
        || Ok(()),
    )
    .expect_err("restore failure should be fatal");

    assert_eq!(
        error,
        PickFilePathError::TerminalState("enter failed".to_string())
    );
}

#[test]
fn pick_file_path_restores_keyboard_enhancement_state_after_selection() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let shared = events.clone();

    let _ = pick_file_path_with_ops(
        |keyboard_enhancement_enabled| {
            shared
                .borrow_mut()
                .push(format!("leave:{keyboard_enhancement_enabled}"));
            Ok(())
        },
        || Ok(None),
        || {
            shared.borrow_mut().push("enter:true".to_string());
            Ok(true)
        },
        || {
            shared.borrow_mut().push("clear".to_string());
            Ok(())
        },
    )
    .expect("picker should succeed");

    assert_eq!(
        events.borrow().as_slice(),
        ["leave:false", "enter:true", "clear"]
    );
}
