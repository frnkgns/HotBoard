use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};

use crate::model::Library;

const LIBRARY_FILE: &str = "library.json";
const SOUNDS_DIR: &str = "sounds";
const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "wav"];

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not locate the app data folder: {error}"))?;
    fs::create_dir_all(dir.join(SOUNDS_DIR))
        .map_err(|error| format!("Could not create the sounds folder: {error}"))?;
    Ok(dir)
}

pub fn sound_path(app: &AppHandle, file_name: &str) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join(SOUNDS_DIR).join(file_name))
}

pub fn load(app: &AppHandle) -> Result<Library, String> {
    let path = data_dir(app)?.join(LIBRARY_FILE);
    if !path.exists() {
        return Ok(Library::default());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("Could not read the saved sound library: {error}"))?;
    serde_json::from_str(&raw).map_err(|error| {
        format!("The saved sound library is unreadable ({error}). Repair or delete it in {LIBRARY_FILE} and restart the app.")
    })
}

pub fn save(app: &AppHandle, library: &Library) -> Result<(), String> {
    let dir = data_dir(app)?;
    let path = dir.join(LIBRARY_FILE);
    let temp = dir.join(format!("{LIBRARY_FILE}.tmp"));
    let json = serde_json::to_string_pretty(library)
        .map_err(|error| format!("Could not encode the sound library: {error}"))?;
    fs::write(&temp, json).map_err(|error| format!("Could not write the sound library: {error}"))?;
    fs::rename(&temp, &path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        format!("Could not save the sound library: {error}")
    })
}

pub fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:032x}{sequence:08x}")
}

fn extension_of(path: &Path) -> Result<String, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();
    if SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        Ok(extension)
    } else {
        Err(format!(
            "{extension} files are not supported. Choose an MP3 or WAV file."
        ))
    }
}

/// Copies the chosen file into app-managed storage and returns the managed file
/// name. The original is left untouched.
pub fn import_audio(app: &AppHandle, id: &str, source: &Path) -> Result<String, String> {
    let source = fs::canonicalize(source)
        .map_err(|error| format!("The selected audio file could not be read: {error}"))?;
    if !source.is_file() {
        return Err("The selected audio file could not be read.".to_string());
    }
    let extension = extension_of(&source)?;
    let file_name = format!("{id}.{extension}");
    let destination = sound_path(app, &file_name)?;
    fs::copy(&source, &destination).map_err(|error| {
        format!(
            "The audio file could not be copied into the app folder: {error}"
        )
    })?;
    Ok(file_name)
}

pub fn remove_audio(app: &AppHandle, file_name: &str) {
    if file_name.is_empty() {
        return;
    }
    if let Ok(path) = sound_path(app, file_name) {
        let _ = fs::remove_file(path);
    }
}

/// Deletes managed audio files that no saved sound points at any more.
pub fn prune_orphans(app: &AppHandle, library: &Library) {
    let Ok(dir) = data_dir(app) else { return };
    let Ok(entries) = fs::read_dir(dir.join(SOUNDS_DIR)) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !library.sounds.iter().any(|sound| sound.file_name == name) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_across_calls() {
        let first = new_id();
        let second = new_id();
        assert_ne!(first, second);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn extensions_are_limited_to_mp3_and_wav() {
        assert_eq!(extension_of(Path::new("a.MP3")).unwrap(), "mp3");
        assert_eq!(extension_of(Path::new("a.wav")).unwrap(), "wav");
        assert!(extension_of(Path::new("a.ogg")).is_err());
        assert!(extension_of(Path::new("a")).is_err());
    }
}