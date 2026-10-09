mod audio;
mod model;
mod sequence;
mod shortcut;
mod store;
mod tray;

#[cfg(windows)]
pub mod hook;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, Runtime, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use audio::SoundPlayer;
use model::{Binding, Library, RegistrationIssue, Sequence, Settings, ShortcutCandidate, ShortcutCheck, Snapshot, Sound, SoundView};

const PLAYBACK_ERROR: &str = "soundboard://playback-error";
const MAX_NAME_LENGTH: usize = 80;

pub struct AppState {
    library: Mutex<Library>,
    bindings: Mutex<HashMap<Shortcut, String>>,
    issues: Mutex<Vec<RegistrationIssue>>,
    player: Option<SoundPlayer>,
    audio_error: Option<String>,
    library_error: Option<String>,
    #[cfg(windows)]
    hook: Option<hook::Hook>,
    hook_error: Option<String>,
    quitting: AtomicBool,
}

fn lock_failed() -> String {
    "The sound library is busy. Try again.".to_string()
}

/// The sequence bindings currently in the library, as `(sound id, steps)`.
fn configured_sequences(state: &AppState) -> Vec<(String, Sequence)> {
    let library = state.library.lock().unwrap_or_else(|error| error.into_inner());
    library
        .sounds
        .iter()
        .filter_map(|sound| {
            sound
                .sequence
                .as_ref()
                .filter(|steps| steps.len() > 1)
                .map(|steps| (sound.id.clone(), steps.clone()))
        })
        .collect()
}

#[cfg(windows)]
fn start_sequence_listener(app: &AppHandle) -> (Option<hook::Hook>, Option<String>) {
    let (tx, rx) = std::sync::mpsc::channel::<Binding>();
    let handle = app.clone();

    if std::thread::Builder::new()
        .name("hotboard-sequence-match".to_string())
        .spawn(move || {
            let mut matcher = sequence::Matcher::new();
            while let Ok(binding) = rx.recv() {
                let sequences = configured_sequences(handle.state::<AppState>().inner());
                if let Some(sound_id) = matcher.push(binding, &sequences) {
                    play(&handle, &sound_id);
                }
            }
        })
        .is_err()
    {
        return (None, Some("Could not start the key sequence listener.".to_string()));
    }

    match hook::Hook::spawn(tx) {
        Ok(hook) => (Some(hook), None),
        Err(message) => (None, Some(message)),
    }
}

/// Exposed so integration tests can prove the hook really installs on Windows.
#[cfg(windows)]
pub mod testing {
    pub fn spawn_sequence_hook(
        sender: std::sync::mpsc::Sender<crate::model::Binding>,
    ) -> Result<crate::hook::Hook, String> {
        crate::hook::Hook::spawn(sender)
    }
}

#[cfg(not(windows))]
fn start_sequence_listener(
    _app: &AppHandle,
) -> (Option<std::convert::Infallible>, Option<String>) {
    (
        None,
        Some("Key sequences are only supported on Windows.".to_string()),
    )
}

fn snapshot(state: &AppState) -> Snapshot {
    let library = state.library.lock().unwrap_or_else(|error| error.into_inner());
    Snapshot {
        sounds: library.sounds.iter().map(SoundView::from).collect(),
        settings: library.settings.clone(),
        issues: state
            .issues
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone(),
        audio_error: state.audio_error.clone(),
        library_error: state.library_error.clone(),
        hook_error: state.hook_error.clone(),
    }
}

fn base_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

fn display_name(name: &str, fallback_path: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Path::new(fallback_path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Sound".to_string());
    }
    trimmed.chars().take(MAX_NAME_LENGTH).collect()
}

fn owner_of<'a>(sounds: &'a [Sound], binding: &Binding, ignore: Option<&str>) -> Option<&'a str> {
    sounds
        .iter()
        .filter(|sound| Some(sound.id.as_str()) != ignore)
        .find(|sound| sound.binding == *binding)
        .map(|sound| sound.name.as_str())
}

fn validate_binding(
    sounds: &[Sound],
    binding: &Binding,
    ignore: Option<&str>,
) -> Result<ShortcutCheck, String> {
    let owner = owner_of(sounds, binding, ignore).map(str::to_string);
    let check = shortcut::check(binding, owner.as_deref());
    if check.ok {
        Ok(check)
    } else {
        Err(check.error.unwrap_or_else(|| "That shortcut cannot be used.".to_string()))
    }
}

fn bind_sound(app: &AppHandle, sound: &Sound) -> Result<(), String> {
    let accelerator = shortcut::accelerator(&sound.binding)?;
    let parsed = shortcut::parse(&accelerator)?;
    app.global_shortcut().register(parsed).map_err(|error| {
        format!(
            "{} could not be registered. Another program is already using it. ({error})",
            sound.shortcut_label()
        )
    })?;
    app.state::<AppState>()
        .bindings
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(parsed, sound.id.clone());
    Ok(())
}

fn unbind_sound(app: &AppHandle, sound: &Sound) {
    let Ok(accelerator) = shortcut::accelerator(&sound.binding) else {
        return;
    };
    let Ok(parsed) = shortcut::parse(&accelerator) else {
        return;
    };
    let _ = app.global_shortcut().unregister(parsed);
    app.state::<AppState>()
        .bindings
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&parsed);
}

/// Re-registers every saved sound and records the ones Windows refused.
fn register_all(app: &AppHandle) {
    let state = app.state::<AppState>();
    let sounds = state
        .library
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .sounds
        .clone();

    let mut bindings = HashMap::new();
    let mut issues = Vec::new();

    for sound in &sounds {
        let label = sound.shortcut_label();
        let attempt = shortcut::accelerator(&sound.binding)
            .and_then(|accelerator| shortcut::parse(&accelerator))
            .and_then(|parsed| {
                app.global_shortcut()
                    .register(parsed)
                    .map(|()| parsed)
                    .map_err(|error| {
                        format!(
                            "Windows refused {label}. Another program is already using it. ({error})"
                        )
                    })
            });
        match attempt {
            Ok(parsed) => {
                bindings.insert(parsed, sound.id.clone());
            }
            Err(message) => issues.push(RegistrationIssue {
                sound_id: sound.id.clone(),
                sound_name: sound.name.clone(),
                shortcut: label,
                message,
            }),
        }
    }

    *state
        .bindings
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = bindings;
    *state
        .issues
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = issues;
}

fn play(app: &AppHandle, sound_id: &str) {
    let state = app.state::<AppState>();
    let Some(player) = state.player.as_ref() else {
        return;
    };
    let path = state
        .library
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .sounds
        .iter()
        .find(|sound| sound.id == sound_id)
        .and_then(|sound| store::sound_path(app, &sound.file_name).ok());

    let Some(path) = path else {
        return;
    };
    if let Err(message) = player.play(sound_id, &path) {
        let _ = app.emit(PLAYBACK_ERROR, message);
    }
}

fn trigger(app: &AppHandle, shortcut: Shortcut) {
    let state = app.state::<AppState>();
    let sound_id = state
        .bindings
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(&shortcut)
        .cloned();
    if let Some(sound_id) = sound_id {
        play(app, &sound_id);
    }
}

pub fn shutdown<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    if state.quitting.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(player) = state.player.as_ref() {
        player.stop_all();
    }
    let active: Vec<Shortcut> = state
        .bindings
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .keys()
        .copied()
        .collect();
    for shortcut in active {
        let _ = app.global_shortcut().unregister(shortcut);
    }
    state
        .bindings
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clear();
    app.exit(0);
}

#[tauri::command]
fn load_library(app: AppHandle) -> Snapshot {
    snapshot(app.state::<AppState>().inner())
}

#[tauri::command]
fn check_shortcut(app: AppHandle, candidate: ShortcutCandidate, ignore_sound_id: Option<String>) -> ShortcutCheck {
    let state = app.state::<AppState>();
    let binding = shortcut::from_candidate(candidate);
    let owner = state
        .library
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .sounds
        .clone();
    let owner = owner_of(&owner, &binding, ignore_sound_id.as_deref()).map(str::to_string);
    shortcut::check(&binding, owner.as_deref())
}

#[tauri::command]
fn check_sequence(
    app: AppHandle,
    steps: Vec<ShortcutCandidate>,
    ignore_sound_id: Option<String>,
) -> sequence::SequenceCheck {
    let state = app.state::<AppState>();
    let candidate: Sequence = steps.into_iter().map(shortcut::from_candidate).collect();
    let sounds = state
        .library
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .sounds
        .clone();

    let owner = sounds
        .iter()
        .filter(|sound| Some(sound.id.as_str()) != ignore_sound_id.as_deref())
        .find(|sound| sound.sequence.as_deref() == Some(candidate.as_slice()))
        .map(|sound| sound.name.clone());

    let mut result = sequence::check(&candidate, owner.as_deref());

    if result.ok && owner.is_none() {
        for sound in &sounds {
            if Some(sound.id.as_str()) == ignore_sound_id.as_deref() {
                continue;
            }
            let Some(existing) = sound.sequence.as_ref() else {
                continue;
            };
            if sequence::is_prefix_of(&candidate, existing)
                || sequence::is_prefix_of(existing, &candidate)
            {
                result.ok = false;
                result.warning = None;
                result.error = Some(format!(
                    "{} overlaps \"{}\" ({}), so only the first steps would ever match.",
                    result.label,
                    sound.name,
                    sequence::label(existing)
                ));
                break;
            }
        }
    }

    result
}

#[tauri::command]
fn add_sound(
    app: AppHandle,
    name: String,
    source_path: String,
    candidate: ShortcutCandidate,
    sequence: Option<Vec<ShortcutCandidate>>,
) -> Result<Snapshot, String> {
    let state = app.state::<AppState>();
    let binding = shortcut::from_candidate(candidate);
    let sequence = sequence
        .map(|steps| {
            steps
                .into_iter()
                .map(shortcut::from_candidate)
                .collect::<Vec<Binding>>()
        })
        .filter(|steps| !steps.is_empty());
    let id = store::new_id();

    let file_name = store::import_audio(&app, &id, Path::new(&source_path))?;
    let sound = Sound {
        id,
        name: display_name(&name, &source_path),
        file_name: file_name.clone(),
        original_name: base_name(&source_path),
        binding,
        sequence: sequence.clone(),
    };

    let outcome = (|| -> Result<(), String> {
        let path = store::sound_path(&app, &file_name)?;
        audio::verify(&path)?;

        let mut library = state.library.lock().map_err(|_| lock_failed())?;
        validate_binding(&library.sounds, &sound.binding, None)?;
        bind_sound(&app, &sound)?;
        library.sounds.push(sound.clone());

        if let Err(error) = store::save(&app, &library) {
            library.sounds.pop();
            unbind_sound(&app, &sound);
            return Err(error);
        }
        Ok(())
    })();

    if let Err(error) = outcome {
        store::remove_audio(&app, &file_name);
        return Err(error);
    }
    Ok(snapshot(state.inner()))
}

#[tauri::command]
fn update_sound(
    app: AppHandle,
    id: String,
    name: String,
    source_path: Option<String>,
    candidate: ShortcutCandidate,
    sequence: Option<Vec<ShortcutCandidate>>,
) -> Result<Snapshot, String> {
    let state = app.state::<AppState>();
    let binding = shortcut::from_candidate(candidate);
    let sequence = sequence
        .map(|steps| {
            steps
                .into_iter()
                .map(shortcut::from_candidate)
                .collect::<Vec<Binding>>()
        })
        .filter(|steps| !steps.is_empty());

    let outcome = (|| -> Result<(), String> {
        let mut library = state.library.lock().map_err(|_| lock_failed())?;
        let index = library
            .sounds
            .iter()
            .position(|sound| sound.id == id)
            .ok_or_else(|| "That sound no longer exists.".to_string())?;
        let previous = library.sounds[index].clone();

        validate_binding(&library.sounds, &binding, Some(&id))?;

        let replacement = match source_path.as_deref() {
            Some(path) => {
                let file_name = store::import_audio(&app, &id, Path::new(path))?;
                let stored = store::sound_path(&app, &file_name)?;
                if let Err(error) = audio::verify(&stored) {
                    store::remove_audio(&app, &file_name);
                    return Err(error);
                }
                Some((file_name, base_name(path)))
            }
            None => None,
        };

        let next = Sound {
            id: previous.id.clone(),
            name: display_name(&name, &previous.original_name),
            file_name: replacement
                .as_ref()
                .map(|(file_name, _)| file_name.clone())
                .unwrap_or_else(|| previous.file_name.clone()),
            original_name: replacement
                .as_ref()
                .map(|(_, original_name)| original_name.clone())
                .unwrap_or_else(|| previous.original_name.clone()),
            binding: binding.clone(),
            sequence: sequence.clone(),
        };

        let binding_changed = previous.binding != binding;
        if binding_changed {
            unbind_sound(&app, &previous);
        }
        if let Err(error) = bind_sound(&app, &next) {
            if binding_changed {
                let _ = bind_sound(&app, &previous);
            }
            if let Some((file_name, _)) = &replacement {
                store::remove_audio(&app, file_name);
            }
            return Err(error);
        }

        library.sounds[index] = next;
        if let Err(error) = store::save(&app, &library) {
            library.sounds[index] = previous;
            register_all(&app);
            return Err(error);
        }

        if let Some((file_name, _)) = &replacement {
            store::remove_audio(&app, &file_name);
        }
        Ok(())
    })();

    outcome?;
    Ok(snapshot(state.inner()))
}

#[tauri::command]
fn delete_sound(app: AppHandle, id: String) -> Result<Snapshot, String> {
    let state = app.state::<AppState>();
    let removed = (|| -> Result<String, String> {
        let mut library = state.library.lock().map_err(|_| lock_failed())?;
        let index = library
            .sounds
            .iter()
            .position(|sound| sound.id == id)
            .ok_or_else(|| "That sound no longer exists.".to_string())?;
        let sound = library.sounds.remove(index);
        unbind_sound(&app, &sound);
        if let Some(player) = state.player.as_ref() {
            player.stop(&sound.id);
        }
        store::save(&app, &library)?;
        Ok(sound.file_name)
    })();

    let removed = removed?;
    store::remove_audio(&app, &removed);

    let library = state.library.lock().map_err(|_| lock_failed())?;
    store::prune_orphans(&app, &library);
    drop(library);

    Ok(snapshot(state.inner()))
}

#[tauri::command]
fn play_sound(app: AppHandle, id: String) {
    play(&app, &id);
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<Snapshot, String> {
    let state = app.state::<AppState>();
    {
        let mut library = state.library.lock().map_err(|_| lock_failed())?;
        library.settings = settings;
        store::save(&app, &library)?;
    }
    Ok(snapshot(state.inner()))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if matches!(event.state(), ShortcutState::Pressed) {
                        trigger(app, *shortcut);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();

            let (library, library_error) = match store::load(&handle) {
                Ok(library) => (library, None),
                Err(message) => (Library::default(), Some(message)),
            };
            let (player, audio_error) = match SoundPlayer::new() {
                Ok(player) => (Some(player), None),
                Err(message) => (None, Some(message)),
            };

            let (hook, hook_error) = start_sequence_listener(&handle);

            app.manage(AppState {
                library: Mutex::new(library),
                bindings: Mutex::new(HashMap::new()),
                issues: Mutex::new(Vec::new()),
                player,
                audio_error,
                library_error,
                hook,
                hook_error,
                quitting: AtomicBool::new(false),
            });

            register_all(&handle);
            tray::build(&handle)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let close_to_tray = app
                    .state::<AppState>()
                    .library
                    .lock()
                    .map(|library| library.settings.close_to_tray)
                    .unwrap_or(true);
                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    shutdown(app);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_library,
            check_shortcut,
            check_sequence,
            add_sound,
            update_sound,
            delete_sound,
            play_sound,
            save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("HotBoard failed to start");
}