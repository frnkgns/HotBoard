import { useCallback, useEffect, useRef, useState } from "react";

import type { ShortcutCandidate } from "../api";

const MODIFIER_KEYS = ["Control", "Alt", "Shift", "Meta", "OS"];
const GAP_MS = 1500;
const MAX_STEPS = 6;

interface Props {
  steps: ShortcutCandidate[];
  empty: boolean;
  hint: string;
  buttonLabel: string;
  onCapture: (steps: ShortcutCandidate[]) => void;
  onClear: () => void;
}

function describe(step: ShortcutCandidate): string {
  const parts: string[] = [];
  if (step.ctrl) parts.push("Ctrl");
  if (step.alt) parts.push("Alt");
  if (step.shift) parts.push("Shift");
  if (step.meta) parts.push("Win");
  const code = step.code;
  if (code.startsWith("Key")) parts.push(code.slice(3));
  else if (code.startsWith("Digit")) parts.push(code.slice(5));
  else if (code.startsWith("Numpad")) parts.push(`Num ${code.slice(6)}`);
  else parts.push(code);
  return parts.join("+");
}

export default function ShortcutRecorder({
  steps,
  empty,
  hint,
  buttonLabel,
  onCapture,
  onClear,
}: Props) {
  const [recording, setRecording] = useState(false);
  const [draft, setDraft] = useState<ShortcutCandidate[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const captureRef = useRef(onCapture);
  const clearRef = useRef(onClear);
  const gapRef = useRef<number | undefined>(undefined);
  const draftRef = useRef<ShortcutCandidate[]>([]);

  captureRef.current = onCapture;
  clearRef.current = onClear;
  draftRef.current = draft;

  const stop = useCallback(() => {
    if (gapRef.current !== undefined) {
      window.clearTimeout(gapRef.current);
      gapRef.current = undefined;
    }
    setRecording(false);
  }, []);

  const commit = useCallback(() => {
    stop();
    const captured = draftRef.current;
    setStatus(null);
    if (captured.length) captureRef.current(captured);
    else setStatus("No key was pressed. Recording stopped.");
  }, [stop]);

  useEffect(() => () => {
    if (gapRef.current !== undefined) window.clearTimeout(gapRef.current);
  }, []);

  function start() {
    setStatus(null);
    draftRef.current = [];
    setDraft([]);
    setRecording(true);
  }

  useEffect(() => {
    if (!recording) return;

    const armGap = () => {
      if (gapRef.current !== undefined) window.clearTimeout(gapRef.current);
      gapRef.current = window.setTimeout(commit, GAP_MS);
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        stop();
        setStatus("Recording cancelled.");
        return;
      }
      if (event.key === "Backspace") {
        const next = draftRef.current.slice(0, -1);
        draftRef.current = next;
        setDraft(next);
        if (!next.length) {
          clearRef.current();
          stop();
          setStatus(null);
        }
        return;
      }
      if (MODIFIER_KEYS.includes(event.key)) {
        armGap();
        return;
      }

      const next = [...draftRef.current, {
        ctrl: event.ctrlKey,
        alt: event.altKey,
        shift: event.shiftKey,
        meta: event.metaKey,
        code: event.code,
      }].slice(-MAX_STEPS);
      draftRef.current = next;
      setDraft(next);
      armGap();
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [recording, stop, commit]);

  const preview = (draft.length ? draft : steps).map(describe).join(" then ");

  return (
    <div className="recorder">
      <div className="recorder__row">
        <button
          type="button"
          className={`bindfield${recording ? " bindfield--armed" : ""}${empty && !draft.length ? " bindfield--empty" : ""}`}
          aria-pressed={recording}
          onClick={() => (recording ? commit() : start())}
        >
          {recording
            ? draft.length
              ? `${preview} …`
              : "Press keys…"
            : preview || "Click to bind a shortcut"}
        </button>
        <button
          type="button"
          className={recording ? "button button--primary recorder__toggle" : "button recorder__toggle"}
          onClick={() => (recording ? commit() : start())}
        >
          {recording ? "Done" : buttonLabel}
        </button>
      </div>
      <p className="recorder__hint" role="status">
        {status ??
          (recording
            ? "Press keys in order, then pause to finish. Backspace removes a step, Escape cancels."
            : hint)}
      </p>
    </div>
  );
}