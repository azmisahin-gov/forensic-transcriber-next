import type { WorkspaceController } from "../../hooks/useWorkspace";

const FORMATS: ReadonlyArray<{ format: string; label: string }> = [
  { format: "TXT", label: "TXT" },
  { format: "SRT", label: "SRT" },
  { format: "JSON", label: "JSON" },
  { format: "HTML", label: "HTML" },
];

/** Export the current transcript and list produced files. */
export function ExportBar({ controller }: { controller: WorkspaceController }) {
  const { transcript, exports, busy } = controller;
  const disabled = busy || !transcript;

  return (
    <section className="panel export-bar" aria-labelledby="export-heading">
      <h2 id="export-heading">Dışa aktarma</h2>
      <div className="row">
        {FORMATS.map(({ format, label }) => (
          <button
            key={format}
            type="button"
            disabled={disabled}
            onClick={() => void controller.exportTranscript(format)}
          >
            {label} olarak dışa aktar
          </button>
        ))}
      </div>

      {exports.length > 0 ? (
        <ul className="export-list">
          {exports.map((record) => (
            <li key={record.id}>
              <span className="badge">{record.format}</span>{" "}
              <span className="export-path">{record.path}</span>
              {record.sha256 ? (
                <span className="export-hash" title={record.sha256}>
                  SHA-256: {record.sha256.slice(0, 12)}…
                </span>
              ) : null}
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}
