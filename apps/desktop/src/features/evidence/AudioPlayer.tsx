import { useEffect, useRef, useState } from "react";

import { mediaSrc } from "../../services/ipc";
import type { EvidenceDto } from "../../types";

/** Format seconds as `mm:ss`. */
function clock(seconds: number): string {
  const safe = Number.isFinite(seconds) ? Math.max(0, seconds) : 0;
  const minutes = Math.floor(safe / 60);
  const secs = Math.floor(safe % 60);
  return `${String(minutes).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

/**
 * Real audio playback of the managed evidence copy via the asset protocol, with
 * play/pause, skip and a seekable timeline. This is the foundation the waveform
 * and segment-to-audio navigation will build on.
 */
export function AudioPlayer({ evidence }: { evidence: EvidenceDto | null }) {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const [playing, setPlaying] = useState(false);
  const [current, setCurrent] = useState(0);
  const [duration, setDuration] = useState(0);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setPlaying(false);
    setCurrent(0);
    setDuration(0);
    setError(null);
  }, [evidence?.id]);

  if (!evidence) {
    return (
      <section className="panel player" aria-labelledby="player-heading">
        <h2 id="player-heading">Ses oynatıcı</h2>
        <p className="empty">Oynatmak için bir kanıt seçin.</p>
      </section>
    );
  }

  const toggle = () => {
    const audio = audioRef.current;
    if (!audio) return;
    if (playing) {
      audio.pause();
    } else {
      void audio.play().catch(() => setError("Oynatma başlatılamadı."));
    }
  };

  const seekBy = (delta: number) => {
    const audio = audioRef.current;
    if (!audio) return;
    audio.currentTime = Math.max(0, Math.min(audio.duration || 0, audio.currentTime + delta));
  };

  return (
    <section className="panel player" aria-labelledby="player-heading">
      <h2 id="player-heading">
        Ses oynatıcı <span className="muted">{evidence.originalName}</span>
      </h2>

      <audio
        ref={audioRef}
        src={mediaSrc(evidence.storedPath)}
        preload="metadata"
        onPlay={() => setPlaying(true)}
        onPause={() => setPlaying(false)}
        onEnded={() => setPlaying(false)}
        onTimeUpdate={(e) => setCurrent(e.currentTarget.currentTime)}
        onLoadedMetadata={(e) => setDuration(e.currentTarget.duration)}
        onError={() =>
          setError(
            "Bu dosya biçimi uygulama içinde oynatılamıyor. Gerekirse FFmpeg ile 16 kHz mono WAV çalışma kopyası oluşturulabilir.",
          )
        }
      />

      <div className="row player-controls">
        <button type="button" onClick={() => seekBy(-5)} aria-label="5 saniye geri">
          −5 sn
        </button>
        <button type="button" className="primary" onClick={toggle}>
          {playing ? "Duraklat" : "Oynat"}
        </button>
        <button type="button" onClick={() => seekBy(5)} aria-label="5 saniye ileri">
          +5 sn
        </button>
        <span className="mono player-time">
          {clock(current)} / {clock(duration)}
        </span>
      </div>

      <label className="sr-only" htmlFor="player-seek">
        Zaman çizgisi
      </label>
      <input
        id="player-seek"
        type="range"
        className="seek"
        min={0}
        max={duration || 0}
        step={0.1}
        value={current}
        onChange={(e) => {
          const audio = audioRef.current;
          const value = Number(e.target.value);
          setCurrent(value);
          if (audio) audio.currentTime = value;
        }}
      />

      {error ? <p className="notice notice-error">{error}</p> : null}
    </section>
  );
}
