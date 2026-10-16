import { useCallback, useEffect, useState } from "react";

import { api, type ApiError } from "../services/ipc";
import type {
  CaseDto,
  EvidenceDto,
  ExportRecordDto,
  SegmentDto,
  TranscriptDto,
} from "../types";

/** UI notice kinds, rendered with distinct styling. */
export type Notice = { kind: "info" | "error"; message: string };

export interface WorkspaceController {
  cases: CaseDto[];
  activeCase: CaseDto | null;
  evidence: EvidenceDto[];
  transcript: TranscriptDto | null;
  segments: SegmentDto[];
  exports: ExportRecordDto[];
  busy: boolean;
  notice: Notice | null;
  refreshCases: () => Promise<void>;
  createCase: (title: string) => Promise<void>;
  openCase: (id: string) => Promise<void>;
  importEvidence: (path: string) => Promise<void>;
  verifyEvidence: (id: string) => Promise<void>;
  transcribe: (evidenceId: string, modelPath: string, useGpu: boolean) => Promise<void>;
  editSegment: (segmentId: string, text: string) => Promise<void>;
  exportTranscript: (format: string) => Promise<void>;
  dismissNotice: () => void;
}

function toNotice(error: unknown): Notice {
  const apiError = error as ApiError;
  const message =
    typeof apiError?.message === "string" ? apiError.message : "Bilinmeyen hata.";
  return { kind: "error", message };
}

/** The single state owner for the workspace view. Business rules live in Rust. */
export function useWorkspace(): WorkspaceController {
  const [cases, setCases] = useState<CaseDto[]>([]);
  const [activeCase, setActiveCase] = useState<CaseDto | null>(null);
  const [evidence, setEvidence] = useState<EvidenceDto[]>([]);
  const [transcript, setTranscript] = useState<TranscriptDto | null>(null);
  const [segments, setSegments] = useState<SegmentDto[]>([]);
  const [exports, setExports] = useState<ExportRecordDto[]>([]);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);

  const refreshCases = useCallback(async () => {
    setCases(await api.listCases());
  }, []);

  useEffect(() => {
    refreshCases().catch((error: unknown) => setNotice(toNotice(error)));
  }, [refreshCases]);

  const loadCaseDetail = useCallback(async (caseDto: CaseDto) => {
    const evidenceList = await api.listEvidence(caseDto.id);
    setEvidence(evidenceList);
    setTranscript(null);
    setSegments([]);
    setExports([]);
  }, []);

  const run = useCallback(
    async (action: () => Promise<void>) => {
      setBusy(true);
      setNotice(null);
      try {
        await action();
      } catch (error) {
        setNotice(toNotice(error));
      } finally {
        setBusy(false);
      }
    },
    [],
  );

  const createCase = useCallback(
    (title: string) =>
      run(async () => {
        const created = await api.createCase(title);
        await refreshCases();
        setActiveCase(created);
        await loadCaseDetail(created);
      }),
    [run, refreshCases, loadCaseDetail],
  );

  const openCase = useCallback(
    (id: string) =>
      run(async () => {
        const caseDto = await api.getCase(id);
        setActiveCase(caseDto);
        await loadCaseDetail(caseDto);
      }),
    [run, loadCaseDetail],
  );

  const importEvidence = useCallback(
    (path: string) =>
      run(async () => {
        if (!activeCase) {
          throw { code: "NO_CASE", message: "Önce bir vaka açın." } satisfies ApiError;
        }
        const imported = await api.importEvidence(activeCase.id, path);
        setEvidence((prev) => [...prev, imported]);
        setNotice({ kind: "info", message: `İçe aktarıldı: ${imported.originalName}` });
      }),
    [run, activeCase],
  );

  const verifyEvidence = useCallback(
    (id: string) =>
      run(async () => {
        const status = await api.verifyEvidence(id);
        setEvidence((prev) =>
          prev.map((e) =>
            e.id === id
              ? { ...e, integrity: status as EvidenceDto["integrity"] }
              : e,
          ),
        );
        setNotice({ kind: "info", message: `Bütünlük durumu: ${status}` });
      }),
    [run],
  );

  const transcribe = useCallback(
    (evidenceId: string, modelPath: string, useGpu: boolean) =>
      run(async () => {
        const outcome = await api.transcribe(evidenceId, modelPath, useGpu);
        setTranscript(outcome.transcript);
        setSegments(outcome.segments);
        const skipped = outcome.skippedSegments;
        setNotice({
          kind: "info",
          message: `Transkripsiyon tamamlandı (${outcome.segments.length} bölüm${
            skipped > 0 ? `, ${skipped} geçersiz bölüm atlandı` : ""
          }).`,
        });
      }),
    [run],
  );

  const editSegment = useCallback(
    (segmentId: string, text: string) =>
      run(async () => {
        const updated = await api.editSegment(segmentId, text);
        setSegments((prev) =>
          prev.map((s) => (s.id === segmentId ? updated : s)),
        );
      }),
    [run],
  );

  const exportTranscript = useCallback(
    (format: string) =>
      run(async () => {
        if (!activeCase || !transcript) {
          throw {
            code: "NO_TRANSCRIPT",
            message: "Dışa aktarılacak transkript yok.",
          } satisfies ApiError;
        }
        const record = await api.exportTranscript(
          activeCase.id,
          transcript.id,
          format,
        );
        setExports((prev) => [...prev, record]);
        setNotice({ kind: "info", message: `Dışa aktarıldı: ${record.path}` });
      }),
    [run, activeCase, transcript],
  );

  const dismissNotice = useCallback(() => setNotice(null), []);

  return {
    cases,
    activeCase,
    evidence,
    transcript,
    segments,
    exports,
    busy,
    notice,
    refreshCases,
    createCase,
    openCase,
    importEvidence,
    verifyEvidence,
    transcribe,
    editSegment,
    exportTranscript,
    dismissNotice,
  };
}
