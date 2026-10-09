//! System-wide keyboard observation, used only to recognise multi-key
//! sequences. Every keystroke is forwarded to Windows untouched: this hook
//! never swallows input.

use std::sync::mpsc::Sender;
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, KBDLLHOOKSTRUCT, LLKHF_INJECTED,
    PostThreadMessageW, SetWindowsHookExW, TranslateMessage, WH_KEYBOARD_LL, MSG, WM_QUIT,
};

use crate::model::Binding;
use crate::shortcut::Modifiers;

const WM_KEYDOWN: usize = 0x0100;
const WM_SYSKEYDOWN: usize = 0x0104;

struct Shared {
    sender: Sender<Binding>,
    modifiers: Mutex<Modifiers>,
}

thread_local! {
    static SHARED: std::cell::Cell<*const Shared> =
        const { std::cell::Cell::new(std::ptr::null()) };
}

fn is_down(vk: i32) -> bool {
    // Bit 31 reports the current state; bit 30 is the toggle state and must
    // not be used here. GetKeyState returns a signed short, so widen first.
    unsafe { (GetKeyState(vk) as u16) & 0x8000 != 0 }
}

fn read_modifiers(mods: &mut Modifiers) {
    mods.ctrl = is_down(VK_CONTROL as i32);
    mods.alt = is_down(VK_MENU as i32);
    mods.shift = is_down(VK_SHIFT as i32);
    mods.meta = is_down(VK_LWIN as i32) || is_down(VK_RWIN as i32);
}

unsafe extern "system" fn callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let forward = || unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) };

    if code != 0 {
        return forward();
    }
    if wparam as usize != WM_KEYDOWN && wparam as usize != WM_SYSKEYDOWN {
        return forward();
    }

    let data = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };

    // Keys synthesised by other processes are ignored so the app cannot
    // retrigger itself.
    if data.flags & LLKHF_INJECTED != 0 {
        return forward();
    }

    let shared = SHARED.with(|cell| cell.get());
    if shared.is_null() {
        return forward();
    }
    let shared = unsafe { &*shared };

    let snapshot = match shared.modifiers.lock() {
        Ok(mut guard) => {
            read_modifiers(&mut guard);
            *guard
        }
        Err(_) => return forward(),
    };

    if let Some(binding) = crate::shortcut::binding_from_vk(data.vkCode as u32, &snapshot) {
        let _ = shared.sender.send(binding);
    }

    forward()
}

/// A running observer. Dropping it stops the thread and removes the hook.
pub struct Hook {
    thread_id: u32,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Hook {
    pub fn spawn(sender: Sender<Binding>) -> Result<Self, String> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<(u32, Result<(), String>)>();

        let join = std::thread::Builder::new()
            .name("hotboard-keyboard".to_string())
            .spawn(move || {
                let thread_id = unsafe { GetCurrentThreadId() };
                let shared = Box::leak(Box::new(Shared {
                    sender,
                    modifiers: Mutex::new(Modifiers::default()),
                }));
                match unsafe { install(shared) } {
                    Ok(()) => {
                        let _ = ready_tx.send((thread_id, Ok(())));
                        unsafe { pump() };
                    }
                    Err(message) => {
                        let _ = ready_tx.send((thread_id, Err(message)));
                    }
                }
            })
            .map_err(|error| format!("Could not start the keyboard listener: {error}"))?;

        match ready_rx.recv() {
            Ok((thread_id, Ok(()))) => Ok(Self {
                thread_id,
                join: Some(join),
            }),
            Ok((_, Err(message))) => {
                let _ = join.join();
                Err(message)
            }
            Err(_) => {
                let _ = join.join();
                Err("The keyboard listener stopped before it started.".to_string())
            }
        }
    }
}

impl Drop for Hook {
    fn drop(&mut self) {
        unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) };
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

unsafe fn install(shared: *const Shared) -> Result<(), String> {
    unsafe {
        SHARED.with(|cell| cell.set(shared));

        let module: HINSTANCE = GetModuleHandleW(std::ptr::null());
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(callback), module, 0);

        if hook.is_null() {
            return Err("Windows refused the keyboard listener. Another program may already be using one.".to_string());
        }
        Ok(())
    }
}

unsafe fn pump() {
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}