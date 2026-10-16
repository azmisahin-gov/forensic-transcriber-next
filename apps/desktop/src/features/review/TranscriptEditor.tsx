import { useState } from "react";

import type { WorkspaceController } from "../../hooks/useWorkspace";
import { formatTime } from "../../services/ipc";
import type { SegmentDto } from "../../types";

const STATUS_LABELS: Record<SegmentDto["status"], string> = {
  AUTOMATIC: "Otomatik",
  REVIEWED: "İncelendi",
  EDITED: "Düzenlendi",
  VERIFIED: "Onaylandı",
};

/** Segment list with inline editing. The automatic text is compared, not lost. */
export function TranscriptEditor({ controller }: { controller: WorkspaceController }) {
  const { segments, transcript, busy } = controller;

  if (!transcript) {
    return (
      <section className="panel transcript-editor" aria-labelledby="transcript-heading">
        <h2 id="transcript-heading">Transkript</h2>
        <p className="empty">
          Transkript yok. Bir kanıt seçip transkripsiyon başlatın.
        </p>
      </section>
    );
  }

  return (
    <section className="panel transcript-editor" aria-labelledby="transcript-heading">
      <h2 id="transcript-heading">
        Transkript <span className="muted">({segments.length} bölüm)</span>
      </h2>
      <ol className="segment-list">
        {segments.map((segment) => (
          <SegmentRow
            key={segment.id}
            segment={segment}
            busy={busy}
            onSave={(text) => controller.editSegment(segment.id, text)}
          />
        ))}
      </ol>
    </section>
  );
}

function SegmentRow({
  segment,
  busy,
  onSave,
}: {
  segment: SegmentDto;
  busy: boolean;
  onSave: (text: string) => Promise<void>;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(segment.text);

  function commit() {
    setEditing(false);
    if (draft !== segment.text) {
      void onSave(draft);
    }
  }

  return (
    <li className="segment" data-status={segment.status}>
      <div className="segment-head">
        <button
          type="button"
          className="segment-time"
          disabled
          title="Ses konumuna gitme (oynatıcı sonraki aşamada)"
        >
          {formatTime(segment.startMs)} – {formatTime(segment.endMs)}
        </button>
        <span className="segment-speaker">{segment.speaker}</span>
        <span className={`badge status-${segment.status.toLowerCase()}`}>
          {STATUS_LABELS[segment.status]}
        </span>
      </div>

      {editing ? (
        <textarea
          className="segment-input"
          value={draft}
          autoFocus
          disabled={busy}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              setDraft(segment.text);
              setEditing(false);
            }
            if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
              commit();
            }
          }}
        />
      ) : (
        <button
          type="button"
          className="segment-text"
          onClick={() => {
            setDraft(segment.text);
            setEditing(true);
          }}
        >
          {segment.text || <span className="muted">(boş)</span>}
        </button>
      )}

      {segment.text !== segment.originalText ? (
        <p className="segment-original" title="Otomatik motor çıktısı">
          Otomatik: {segment.originalText}
        </p>
      ) : null}
    </li>
  );
}
