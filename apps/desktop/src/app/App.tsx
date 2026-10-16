import { useState } from "react";

import { CasePanel } from "../components/CasePanel";
import { StatusBar } from "../components/StatusBar";
import { EvidencePanel } from "../features/evidence/EvidencePanel";
import { ExportBar } from "../features/exports/ExportBar";
import { TranscriptEditor } from "../features/review/TranscriptEditor";
import { TranscriptionPanel } from "../features/transcription/TranscriptionPanel";
import { useWorkspace } from "../hooks/useWorkspace";
import type { EvidenceDto } from "../types";

export function App() {
  const controller = useWorkspace();
  const [selected, setSelected] = useState<EvidenceDto | null>(null);

  return (
    <div className="app-shell">
      <header className="app-header">
        <h1>Forensic Transcriber Next</h1>
        <p className="tagline">
          Yerel · Çevrimdışı · Türkçe öncelikli transkripsiyon ve uzman inceleme
          çalışma alanı
        </p>
      </header>

      <div className="app-body">
        <aside className="app-sidebar">
          <CasePanel controller={controller} />
        </aside>

        <main className="app-main">
          <EvidencePanel
            controller={controller}
            selectedId={selected?.id ?? null}
            onSelect={setSelected}
          />
          <TranscriptionPanel controller={controller} selected={selected} />
          <TranscriptEditor controller={controller} />
          <ExportBar controller={controller} />
        </main>
      </div>

      <StatusBar
        notice={controller.notice}
        busy={controller.busy}
        onDismiss={controller.dismissNotice}
      />
    </div>
  );
}
