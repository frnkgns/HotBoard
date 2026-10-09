import { useCallback, useEffect, useState } from "react";

import {
  deleteSound,
  errorMessage,
  loadLibrary,
  onPlaybackError,
  playSound,
  saveSettings,
} from "./api";
import type { Snapshot, Sound } from "./api";
import ConfirmDialog from "./components/ConfirmDialog";
import SoundEditor from "./components/SoundEditor";
import SoundRow from "./components/SoundRow";

const NOTICE_TIMEOUT_MS = 6000;

export default function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [fatal, setFatal] = useState<string | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editing, setEditing] = useState<Sound | null>(null);
  const [pendingDelete, setPendingDelete] = useState<Sound | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    loadLibrary().then(setSnapshot).catch((cause) => setFatal(errorMessage(cause)));
  }, []);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    let active = true;
    onPlaybackError(setNotice).then((unlisten) => {
      if (active) dispose = unlisten;
      else unlisten();
    });
    return () => {
      active = false;
      dispose?.();
    };
  }, []);

  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(null), NOTICE_TIMEOUT_MS);
    return () => window.clearTimeout(timer);
  }, [notice]);

  const openEditor = useCallback((sound: Sound | null) => {
    setEditing(sound);
    setEditorOpen(true);
  }, []);

  function handlePlay(sound: Sound) {
    playSound(sound.id).catch((cause) => setNotice(errorMessage(cause)));
  }

  async function handleDelete() {
    if (!pendingDelete) return;
    setDeleting(true);
    try {
      setSnapshot(await deleteSound(pendingDelete.id));
      setPendingDelete(null);
    } catch (cause) {
      setNotice(errorMessage(cause));
    } finally {
      setDeleting(false);
    }
  }

  async function handleCloseToTray(closeToTray: boolean) {
    if (!snapshot) return;
    try {
      setSnapshot(await saveSettings({ ...snapshot.settings, closeToTray }));
    } catch (cause) {
      setNotice(errorMessage(cause));
    }
  }

  if (fatal) {
    return (
      <main className="screen screen--centered">
        <div className="banner banner--error" role="alert">
          <strong>The sound library could not be opened.</strong>
          <span>{fatal}</span>
        </div>
      </main>
    );
  }

  if (!snapshot) {
    return (
      <main className="screen screen--centered">
        <p className="muted">Loading your sounds…</p>
      </main>
    );
  }

  const issueFor = (id: string) =>
    snapshot.issues.find((issue) => issue.soundId === id)?.message ?? null;

  return (
    <main className="screen">
      <header className="header">
        <div>
          <h1 className="header__title">HotBoard</h1>
          <p className="header__subtitle">
            {snapshot.sounds.length === 0
              ? "No sounds yet"
              : `${snapshot.sounds.length} sound${snapshot.sounds.length === 1 ? "" : "s"}`}
          </p>
        </div>
        <button type="button" className="button button--primary" onClick={() => openEditor(null)}>
          Add sound
        </button>
      </header>

      {snapshot.audioError ? (
        <div className="banner banner--error" role="alert">
          <strong>Audio output unavailable.</strong>
          <span>{snapshot.audioError}</span>
        </div>
      ) : null}

      {snapshot.libraryError ? (
        <div className="banner banner--error" role="alert">
          <strong>Your saved sounds were not loaded.</strong>
          <span>{snapshot.libraryError}</span>
        </div>
      ) : null}

      {snapshot.hookError ? (
        <div className="banner banner--warning" role="status">
          <strong>Key sequences are unavailable.</strong>
          <span>{snapshot.hookError}</span>
        </div>
      ) : null}

      {snapshot.issues.length > 0 ? (
        <div className="banner banner--warning" role="status">
          <strong>
            {snapshot.issues.length} shortcut{snapshot.issues.length === 1 ? "" : "s"} could not be
            registered.
          </strong>
          <span>{snapshot.issues.map((issue) => `${issue.soundName}: ${issue.shortcut}`).join(", ")}</span>
        </div>
      ) : null}

      {snapshot.sounds.length === 0 ? (
        <section className="empty">
          <p>Import an MP3 or WAV and give it a shortcut. It plays anywhere, even while this app is closed.</p>
        </section>
      ) : (
        <ul className="sounds">
          {snapshot.sounds.map((sound) => (
            <SoundRow
              key={sound.id}
              sound={sound}
              issue={issueFor(sound.id)}
              onPlay={handlePlay}
              onEdit={openEditor}
              onDelete={setPendingDelete}
            />
          ))}
        </ul>
      )}

      <footer className="footer">
        <label className="toggle">
          <input
            type="checkbox"
            checked={snapshot.settings.closeToTray}
            onChange={(event) => handleCloseToTray(event.currentTarget.checked)}
          />
          <span>Closing the window hides it to the system tray</span>
        </label>
        <p className="muted">Use the tray icon to reopen or quit.</p>
      </footer>

      {editorOpen ? (
        <SoundEditor
          sound={editing}
          onCancel={() => setEditorOpen(false)}
          onSaved={(next) => {
            setSnapshot(next);
            setEditorOpen(false);
          }}
        />
      ) : null}

      {pendingDelete ? (
        <ConfirmDialog
          title="Delete this sound?"
          message={`"${pendingDelete.name}" and its shortcut will be removed, and the copied audio file will be deleted. This cannot be undone.`}
          confirmLabel="Delete"
          busy={deleting}
          onConfirm={handleDelete}
          onCancel={() => setPendingDelete(null)}
        />
      ) : null}

      {notice ? (
        <div className="toast" role="status">
          {notice}
        </div>
      ) : null}
    </main>
  );
}