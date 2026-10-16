import { useCallback, useEffect, useState } from "react";

import {
  api,
  onTranscriptionProgress,
  onTranscriptionStage,
  type ApiError,
  type StartupStatusDto,
} from "../services/ipc";
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
  /** Id of the running transcription job, when one is in progress. */
  activeJobId: string | null;
  /** Latest progress fraction in [0, 1], or null when unknown. */
  progress: number | null;
  /** Startup diagnostics (workspace + engine availability). */
  startup: StartupStatusDto | null;
  notice: Notice | null;
  refreshCases: () => Promise<void>;
  createCase: (title: string) => Promise<void>;
  openCase: (id: string) => Promise<void>;
  importEvidence: (path: string) => Promise<void>;
  verifyEvidence: (id: string) => Promise<void>;
  transcribe: (evidenceId: string, modelPath: string, useGpu: boolean) => Promise<void>;
  cancelTranscription: () => Promise<void>;
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
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [startup, setStartup] = useState<StartupStatusDto | null>(null);

  const refreshCases = useCallback(async () => {
    setCases(await api.listCases());
  }, []);

  useEffect(() => {
    refreshCases().catch((error: unknown) => setNotice(toNotice(error)));
  }, [refreshCases]);

  // Load startup diagnostics (workspace + engine availability).
  useEffect(() => {
    api
      .startupStatus()
      .then(setStartup)
      .catch(() => setStartup(null));
  }, []);

  // Subscribe to the Rust core's transcription events for the component's life.
  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;
    const track = (registration: Promise<() => void>) => {
      void registration.then((unlisten) => {
        if (disposed) unlisten();
        else unlisteners.push(unlisten);
      });
    };
    track(
      onTranscriptionProgress((event) => {
        setActiveJobId(event.jobId);
        setProgress(event.fraction);
      }),
    );
    track(
      onTranscriptionStage((event) => {
        setActiveJobId(event.jobId);
        if (event.stage === "finished") {
          setActiveJobId(null);
          setProgress(null);
        }
      }),
    );
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

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
        setActiveJobId(null);
        setProgress(null);
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

  const cancelTranscription = useCallback(
    (): Promise<void> =>
      run(async () => {
        if (!activeJobId) {
          setNotice({ kind: "info", message: "İptal edilecek etkin iş yok." });
          return;
        }
        const requested = await api.cancelTranscription(activeJobId);
        setNotice({
          kind: "info",
          message: requested ? "İptal istendi." : "İş bulunamadı.",
        });
      }),
    [run, activeJobId],
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
    activeJobId,
    progress,
    startup,
    notice,
    refreshCases,
    createCase,
    openCase,
    importEvidence,
    verifyEvidence,
    transcribe,
    cancelTranscription,
    editSegment,
    exportTranscript,
    dismissNotice,
  };
}
