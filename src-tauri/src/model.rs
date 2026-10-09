use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Binding {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub code: String,
}

pub type Sequence = Vec<Binding>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub file_name: String,
    pub original_name: String,
    pub binding: Binding,
    #[serde(default)]
    pub sequence: Option<Sequence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub close_to_tray: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub sounds: Vec<Sound>,
    pub settings: Settings,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            sounds: Vec::new(),
            settings: Settings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundView {
    pub id: String,
    pub name: String,
    pub shortcut: String,
    pub sequence_label: Option<String>,
    pub original_name: String,
    pub binding: Binding,
}

impl From<&Sound> for SoundView {
    fn from(sound: &Sound) -> Self {
        Self {
            id: sound.id.clone(),
            name: sound.name.clone(),
            shortcut: sound.shortcut_label(),
            sequence_label: sound
                .sequence
                .as_ref()
                .filter(|steps| steps.len() > 1)
                .map(|steps| crate::sequence::label(steps)),
            original_name: sound.original_name.clone(),
            binding: sound.binding.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationIssue {
    pub sound_id: String,
    pub sound_name: String,
    pub shortcut: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub sounds: Vec<SoundView>,
    pub settings: Settings,
    pub issues: Vec<RegistrationIssue>,
    pub audio_error: Option<String>,
    pub library_error: Option<String>,
    #[serde(rename = "hookError")]
    pub hook_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutCandidate {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub code: String,
}

impl From<ShortcutCandidate> for Binding {
    fn from(candidate: ShortcutCandidate) -> Self {
        Self {
            ctrl: candidate.ctrl,
            alt: candidate.alt,
            shift: candidate.shift,
            meta: candidate.meta,
            code: candidate.code,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutCheck {
    pub shortcut: String,
    pub ok: bool,
    pub warning: Option<String>,
    pub error: Option<String>,
}

impl ShortcutCheck {
    pub fn rejected(message: impl Into<String>) -> Self {
        Self {
            shortcut: String::new(),
            ok: false,
            warning: None,
            error: Some(message.into()),
        }
    }
}