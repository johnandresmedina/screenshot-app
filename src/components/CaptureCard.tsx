import { imageUrl, type Capture } from "../lib/api";
import { sizeLabel, timeLabel } from "../lib/format";

interface Props {
  capture: Capture;
  onCopy: (c: Capture) => void;
  onOpen: (c: Capture) => void;
  onReveal: (c: Capture) => void;
  onDelete: (c: Capture) => void;
}

export function CaptureCard({ capture: c, onCopy, onOpen, onReveal, onDelete }: Props) {
  const dims = c.width && c.height ? `${c.width}×${c.height}` : null;
  return (
    <article className="card">
      <button className="thumb" onClick={() => onOpen(c)} title={`Open ${c.id}`}>
        <img src={imageUrl(c)} alt={c.id} loading="lazy" draggable={false} />
        {c.source === "mcp" && <span className="badge">Claude</span>}
      </button>
      <div className="meta">
        <span className="time">{timeLabel(c.createdAt)}</span>
        <span className="details">{[dims, sizeLabel(c.sizeBytes)].filter(Boolean).join(" · ")}</span>
      </div>
      <div className="card-actions">
        <button className="ghost small" onClick={() => onCopy(c)}>Copy</button>
        <button className="ghost small" onClick={() => onOpen(c)}>Open</button>
        <button className="ghost small" onClick={() => onReveal(c)} title="Show in Finder">Finder</button>
        <button className="ghost small danger" onClick={() => onDelete(c)} title="Move to Trash">Delete</button>
      </div>
    </article>
  );
}
