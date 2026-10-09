use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Mutex;

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

pub struct SoundPlayer {
    sink: MixerDeviceSink,
    players: Mutex<HashMap<String, Player>>,
}

impl SoundPlayer {
    pub fn new() -> Result<Self, String> {
        let sink = DeviceSinkBuilder::open_default_sink()
            .map_err(|error| format!("No audio output device is available: {error}"))?;
        Ok(Self {
            sink,
            players: Mutex::new(HashMap::new()),
        })
    }

    /// Restarts `key` from the beginning. Pressing the same shortcut again
    /// interrupts the previous playback instead of layering on top of it.
    pub fn play(&self, key: &str, path: &Path) -> Result<(), String> {
        self.stop(key);

        let name = path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| "the saved file".to_string());
        let file =
            File::open(path).map_err(|error| format!("{name} could not be opened: {error}"))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|_| "This audio file could not be decoded. Use an MP3 or WAV file.".to_string())?;

        let mixer = self.sink.mixer();
        let player = Player::connect_new(&mixer);
        player.append(source);
        player.play();

        self.prune();
        self.players
            .lock()
            .map_err(|_| "Playback state is unavailable.".to_string())?
            .insert(key.to_string(), player);
        Ok(())
    }

    pub fn stop(&self, key: &str) {
        if let Ok(mut players) = self.players.lock() {
            if let Some(player) = players.remove(key) {
                player.stop();
            }
        }
    }

    pub fn stop_all(&self) {
        if let Ok(mut players) = self.players.lock() {
            for (_, player) in players.drain() {
                player.stop();
            }
        }
    }

    fn prune(&self) {
        if let Ok(mut players) = self.players.lock() {
            players.retain(|_, player| !player.empty());
        }
    }
}

/// Confirms a file can actually be decoded before it is stored, so a broken
/// import is reported at import time instead of the first time it is played.
pub fn verify(path: &Path) -> Result<(), String> {
    let file =
        File::open(path).map_err(|error| format!("The audio file could not be read: {error}"))?;
    Decoder::new(BufReader::new(file))
        .map(|_| ())
        .map_err(|_| "This file is not a supported MP3 or WAV recording.".to_string())
}