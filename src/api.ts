import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

export interface ShortcutCandidate {
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  meta: boolean;
  code: string;
}

export interface Sound {
  id: string;
  name: string;
  shortcut: string;
  sequenceLabel: string | null;
  originalName: string;
  binding: ShortcutCandidate;
}

export interface Settings {
  closeToTray: boolean;
}

export interface RegistrationIssue {
  soundId: string;
  soundName: string;
  shortcut: string;
  message: string;
}

export interface Snapshot {
  sounds: Sound[];
  settings: Settings;
  issues: RegistrationIssue[];
  audioError: string | null;
  libraryError: string | null;
  hookError: string | null;
}

export interface ShortcutCheck {
  shortcut: string;
  ok: boolean;
  warning: string | null;
  error: string | null;
}

export interface SequenceCheck {
  label: string;
  ok: boolean;
  warning: string | null;
  error: string | null;
}

const PLAYBACK_ERROR = "soundboard://playback-error";

export function errorMessage(cause: unknown): string {
  if (typeof cause === "string") return cause;
  if (cause instanceof Error) return cause.message;
  return "Something went wrong.";
}

export function loadLibrary(): Promise<Snapshot> {
  return invoke<Snapshot>("load_library");
}

export function checkShortcut(
  candidate: ShortcutCandidate,
  ignoreSoundId?: string,
): Promise<ShortcutCheck> {
  return invoke<ShortcutCheck>("check_shortcut", {
    candidate,
    ignoreSoundId: ignoreSoundId ?? null,
  });
}

export function checkSequence(
  steps: ShortcutCandidate[],
  ignoreSoundId?: string,
): Promise<SequenceCheck> {
  return invoke<SequenceCheck>("check_sequence", {
    steps,
    ignoreSoundId: ignoreSoundId ?? null,
  });
}

export function addSound(
  name: string,
  sourcePath: string,
  steps: ShortcutCandidate[],
): Promise<Snapshot> {
  const [first, ...rest] = steps;
  return invoke<Snapshot>("add_sound", {
    name,
    sourcePath,
    candidate: first,
    sequence: rest.length ? steps : null,
  });
}

export function updateSound(
  id: string,
  name: string,
  sourcePath: string | null,
  steps: ShortcutCandidate[],
): Promise<Snapshot> {
  const [first, ...rest] = steps;
  return invoke<Snapshot>("update_sound", {
    id,
    name,
    sourcePath,
    candidate: first,
    sequence: rest.length ? steps : null,
  });
}

export function deleteSound(id: string): Promise<Snapshot> {
  return invoke<Snapshot>("delete_sound", { id });
}

export function playSound(id: string): Promise<void> {
  return invoke<void>("play_sound", { id });
}

export function saveSettings(settings: Settings): Promise<Snapshot> {
  return invoke<Snapshot>("save_settings", { settings });
}

export async function pickAudioFile(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Audio", extensions: ["mp3", "wav"] }],
  });
  if (typeof selected === "string") return selected;
  if (Array.isArray(selected) && typeof selected[0] === "string") return selected[0];
  return null;
}

export async function onPlaybackError(
  handler: (message: string) => void,
): Promise<() => void> {
  return listen<string>(PLAYBACK_ERROR, (event) => handler(event.payload));
}

export function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

export function stripExtension(fileName: string): string {
  const dot = fileName.lastIndexOf(".");
  return dot > 0 ? fileName.slice(0, dot) : fileName;
}