import { useCallback, useEffect, useState } from "react";

import {
  api,
  onRuntimeProgress,
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
  /** True while an engine/model download is running. */
  runtimeBusy: boolean;
  /** Latest runtime download fraction in [0,1], or null when unknown. */
  runtimeProgress: number | null;
  prepareEngine: () => Promise<void>;
  downloadModel: (modelId: string) => Promise<void>;
  cancelPreparation: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  notice: Notice | null;
  refreshCases: () => Promise<void>;
  createCase: (title: string) => Promise<void>;
  openCase: (id: string) => Promise<void>;
  /** Load the stored transcript for an evidence, if one exists. */
  loadEvidenceTranscript: (evidenceId: string) => Promise<void>;
  importEvidence: (path: string) => Promise<void>;
  verifyEvidence: (id: string) => Promise<void>;
  transcribe: (evidenceId: string, modelPath: string | null, useGpu: boolean, language?: string) => Promise<void>;
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
  const [runtimeBusy, setRuntimeBusy] = useState(false);
  const [runtimeProgress, setRuntimeProgress] = useState<number | null>(null);

  const refreshCases = useCallback(async () => {
    setCases(await api.listCases());
  }, []);

  useEffect(() => {
    refreshCases().catch((error: unknown) => setNotice(toNotice(error)));
  }, [refreshCases]);

  const refreshStatus = useCallback(async () => {
    setStartup(await api.startupStatus());
  }, []);

  useEffect(() => {
    refreshStatus().catch(() => setStartup(null));
  }, [refreshStatus]);

  // Runtime (engine/model) download progress.
  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;
    void onRuntimeProgress((event) => {
      setRuntimeProgress(event.total > 0 ? event.downloaded / event.total : null);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    });
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
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

  const loadEvidenceTranscript = useCallback(
    (evidenceId: string) =>
      run(async () => {
        const existing = await api.transcriptForEvidence(evidenceId);
        if (existing) {
          setTranscript(existing);
          setSegments(await api.listSegments(existing.id));
        } else {
          setTranscript(null);
          setSegments([]);
        }
      }),
    [run],
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
    (evidenceId: string, modelPath: string | null, useGpu: boolean, language?: string) =>
      run(async () => {
        const outcome = await api.transcribe(evidenceId, modelPath, useGpu, language);
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

  const prepareEngine = useCallback(
    (): Promise<void> =>
      run(async () => {
        setRuntimeBusy(true);
        try {
          const status = await api.prepareEngine();
          setStartup(status);
          setNotice({ kind: "info", message: "Transkripsiyon motoru kuruldu." });
        } finally {
          setRuntimeBusy(false);
          setRuntimeProgress(null);
        }
      }),
    [run],
  );

  const downloadModel = useCallback(
    (modelId: string): Promise<void> =>
      run(async () => {
        setRuntimeBusy(true);
        try {
          const status = await api.downloadModel(modelId);
          setStartup(status);
          setNotice({ kind: "info", message: "Model indirildi ve doğrulandı." });
        } finally {
          setRuntimeBusy(false);
          setRuntimeProgress(null);
        }
      }),
    [run],
  );

  const cancelPreparation = useCallback(
    (): Promise<void> =>
      run(async () => {
        const requested = await api.cancelPreparation();
        setNotice({
          kind: "info",
          message: requested ? "İndirme iptal edildi." : "Etkin indirme yok.",
        });
      }),
    [run],
  );

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
    runtimeBusy,
    runtimeProgress,
    prepareEngine,
    downloadModel,
    cancelPreparation,
    refreshStatus,
    notice,
    refreshCases,
    createCase,
    openCase,
    loadEvidenceTranscript,
    importEvidence,
    verifyEvidence,
    transcribe,
    cancelTranscription,
    editSegment,
    exportTranscript,
    dismissNotice,
  };
}
