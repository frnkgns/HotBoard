import { useState } from "react";

import {
  addSound,
  baseName,
  checkSequence,
  errorMessage,
  pickAudioFile,
  stripExtension,
  updateSound,
} from "../api";
import type { SequenceCheck, ShortcutCandidate, Snapshot, Sound } from "../api";
import ShortcutRecorder from "./ShortcutRecorder";

interface Props {
  sound: Sound | null;
  onCancel: () => void;
  onSaved: (snapshot: Snapshot) => void;
}

export default function SoundEditor({ sound, onCancel, onSaved }: Props) {
  const isNew = sound === null;
  const [name, setName] = useState(sound?.name ?? "");
  const [replacement, setReplacement] = useState<string | null>(null);
  const [steps, setSteps] = useState<ShortcutCandidate[]>(
    sound ? (sound.sequenceLabel ? [] : [sound.binding]) : [],
  );
  const [check, setCheck] = useState<SequenceCheck | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const audioLabel = replacement ? baseName(replacement) : (sound?.originalName ?? null);
  const shortcutLabel = check?.ok === false ? "Not available" : (check?.label ?? "");
  const blocked = steps.length === 0 || (check !== null && !check.ok);

  async function chooseFile() {
    setError(null);
    try {
      const picked = await pickAudioFile();
      if (!picked) return;
      setReplacement(picked);
      if (isNew) setName(stripExtension(baseName(picked)));
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }

  async function handleCapture(next: ShortcutCandidate[]) {
    setSteps(next);
    try {
      setCheck(await checkSequence(next, sound?.id));
    } catch (cause) {
      setCheck({ label: "", ok: false, warning: null, error: errorMessage(cause) });
    }
  }

  function handleClear() {
    setSteps([]);
    setCheck(null);
  }

  async function save() {
    if (!steps.length) {
      setError("Record a shortcut before saving.");
      return;
    }
    if (isNew && !replacement) {
      setError("Choose an MP3 or WAV file first.");
      return;
    }

    setBusy(true);
    setError(null);
    try {
      const snapshot = sound
        ? await updateSound(sound.id, name, replacement, steps)
        : await addSound(name, replacement as string, steps);
      onSaved(snapshot);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="overlay" role="presentation" onClick={() => { if (!busy) onCancel(); }}>
      <div
        className="panel"
        role="dialog"
        aria-modal="true"
        aria-label={isNew ? "Add sound" : "Edit sound"}
        onClick={(event) => event.stopPropagation()}
      >
        <h2 className="panel__title">{isNew ? "Add sound" : "Edit sound"}</h2>

        <div className="field">
          <label className="field__label" htmlFor="sound-name">
            Name
          </label>
          <input
            id="sound-name"
            className="field__input"
            value={name}
            maxLength={80}
            disabled={busy}
            onChange={(event) => setName(event.currentTarget.value)}
            placeholder="Sound name"
          />
        </div>

        <div className="field">
          <span className="field__label">Audio file</span>
          <div className="field__row">
            <span className={audioLabel ? "field__value" : "field__value field__value--empty"}>
              {audioLabel ?? "No file selected"}
            </span>
            <button type="button" className="button" onClick={chooseFile} disabled={busy}>
              {isNew ? "Choose file…" : "Replace file…"}
            </button>
          </div>
          <p className="field__hint">MP3 or WAV. The file is copied into the app folder.</p>
        </div>

        <div className="field">
          <span className="field__label">Shortcut</span>
          <ShortcutRecorder
            steps={steps}
            empty={!shortcutLabel}
            hint="One key with modifiers, or several keys in a row. Add Ctrl, Alt, Shift or Win to reduce clashes."
            buttonLabel={steps.length ? "Re-record" : "Record"}
            onCapture={handleCapture}
            onClear={handleClear}
          />
          {check?.error ? <p className="field__error">{check.error}</p> : null}
          {check?.warning ? <p className="field__warning">{check.warning}</p> : null}
        </div>

        {error ? (
          <p className="field__error" role="alert">
            {error}
          </p>
        ) : null}

        <div className="panel__actions">
          <button type="button" className="button" onClick={onCancel} disabled={busy}>
            Cancel
          </button>
          <button
            type="button"
            className="button button--primary"
            onClick={save}
            disabled={busy || blocked}
          >
            {busy ? "Saving…" : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}