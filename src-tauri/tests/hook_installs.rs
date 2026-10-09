use std::sync::mpsc::channel;

use hotboard_lib::testing::spawn_sequence_hook;

#[test]
fn the_keyboard_hook_installs() {
    let (tx, rx) = channel();
    let hook = match spawn_sequence_hook(tx) {
        Ok(hook) => hook,
        Err(message) => panic!("hook refused to install: {message}"),
    };
    drop(rx);
    drop(hook);
}