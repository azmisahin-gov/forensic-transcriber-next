import { useState } from "react";

import type { WorkspaceController } from "../hooks/useWorkspace";

/** Left rail: create, list and open cases. */
export function CasePanel({ controller }: { controller: WorkspaceController }) {
  const [title, setTitle] = useState("");
  const { cases, activeCase, busy } = controller;

  function submit(event: React.FormEvent) {
    event.preventDefault();
    const trimmed = title.trim();
    if (!trimmed) return;
    void controller.createCase(trimmed).then(() => setTitle(""));
  }

  return (
    <section className="panel case-panel" aria-labelledby="cases-heading">
      <h2 id="cases-heading">Vakalar</h2>

      <form className="row" onSubmit={submit}>
        <label className="sr-only" htmlFor="new-case-title">
          Yeni vaka başlığı
        </label>
        <input
          id="new-case-title"
          type="text"
          placeholder="Yeni vaka başlığı"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          disabled={busy}
        />
        <button type="submit" disabled={busy || title.trim().length === 0}>
          Oluştur
        </button>
      </form>

      {cases.length === 0 ? (
        <p className="empty">Henüz vaka yok. Yukarıdan bir vaka oluşturun.</p>
      ) : (
        <ul className="case-list">
          {cases.map((c) => (
            <li key={c.id}>
              <button
                type="button"
                className={c.id === activeCase?.id ? "case-item active" : "case-item"}
                aria-current={c.id === activeCase?.id}
                onClick={() => void controller.openCase(c.id)}
              >
                <span className="case-title">{c.title}</span>
                <span className="case-meta">
                  Son güncelleme: {new Date(c.updatedAt).toLocaleString("tr-TR")}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
