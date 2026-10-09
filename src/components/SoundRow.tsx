import type { Sound } from "../api";

interface Props {
  sound: Sound;
  issue: string | null;
  onPlay: (sound: Sound) => void;
  onEdit: (sound: Sound) => void;
  onDelete: (sound: Sound) => void;
}

export default function SoundRow({ sound, issue, onPlay, onEdit, onDelete }: Props) {
  return (
    <li className="sound">
      <div className="sound__main">
        <div className="sound__identity">
          <span className="sound__name">{sound.name}</span>
          <span className="sound__file">{sound.originalName}</span>
        </div>
        <kbd className="sound__shortcut">{sound.sequenceLabel ?? sound.shortcut}</kbd>
      </div>
      {issue ? (
        <p className="sound__issue" role="status">
          {issue}
        </p>
      ) : null}
      <div className="sound__actions">
        <button type="button" className="button button--small" onClick={() => onPlay(sound)}>
          Play
        </button>
        <button type="button" className="button button--small" onClick={() => onEdit(sound)}>
          Edit
        </button>
        <button
          type="button"
          className="button button--small button--danger"
          onClick={() => onDelete(sound)}
        >
          Delete
        </button>
      </div>
    </li>
  );
}