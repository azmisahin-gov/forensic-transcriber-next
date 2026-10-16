# Mimari Genel Bakış

## Amaç

Yerel, çevrimdışı, Türkçe öncelikli bir ses transkripsiyonu ve uzman inceleme
çalışma alanı. Veri bütünlüğü ve doğru veri modeli en yüksek önceliktir.

## Katmanlar

```text
┌───────────────────────────────────────────────┐
│ React + TypeScript  (apps/desktop/src)         │
│   - ekranlar, durum yönetimi, erişilebilirlik  │
│   - iş kuralı içermez                          │
└───────────────▲───────────────────────────────┘
                │  Tauri invoke (camelCase DTO)
┌───────────────┴───────────────────────────────┐
│ Tauri command surface (src-tauri/src/commands) │
│   - girdi doğrulama, güvenli hata dönüşü       │
└───────────────▲───────────────────────────────┘
                │
┌───────────────┴───────────────────────────────┐
│ ftn-application  (Workspace, use case'ler)     │
└───┬───────┬────────┬──────────┬────────────────┘
    │       │        │          │
 ftn-domain ftn-    ftn-media  ftn-transcription  ftn-export
 (kurallar) storage  (FFmpeg)   (EngineAdapter)    (TXT/SRT/JSON/HTML)
            (SQLite)
```

## Sorumluluklar

- **ftn-domain** — `Case`, `EvidenceFile`, `DerivedMediaAsset`, `TranscriptionJob`,
  `Transcript`, `TranscriptRevision`, `TranscriptSegment`, `SpeakerLabel`,
  `AuditEvent`, `ExportRecord`, `ApplicationSetting`. SQLite, FFmpeg veya UI'a
  bağımlı değildir.
- **ftn-application** — `Workspace` üzerinden kullanıcı işlemleri: vaka oluşturma,
  kanıt içe aktarma, bütünlük doğrulama, transkripsiyon, segment düzenleme, dışa
  aktarma. İş kurallarının tek yetkili uygulama katmanı.
- **ftn-storage** — SQLite şeması, sürümlenmiş migrasyonlar, parametreli SQL,
  transaction kullanımı, foreign key.
- **ftn-media** — FFmpeg/FFprobe sarmalayıcı; güvenli süreç başlatma, 16 kHz mono
  WAV üretimi, medya metadata okuma.
- **ftn-transcription** — `EngineAdapter` arayüzü ve `WhisperCppAdapter`.
- **ftn-export** — TXT/SRT/JSON/HTML üretimi; şema sürümü ve kaçışlama.
- **src-tauri** — pencere, IPC, DTO dönüşümü, `AppState`.
- **apps/desktop/src** — React ekranları ve `useWorkspace` durum yöneticisi.

## Tek yönlü bağımlılık

`domain` hiçbir şeye bağlı değildir. `application` altyapı crate'lerine bağlıdır.
Tauri ve React yalnızca `application` üzerinden veriye erişir. Böylece aynı kural
iki yerde tekrarlanmaz.

## Hata yönetimi

- Rust'ta `Result` ve `thiserror` tabanlı tipli hatalar.
- Uygulama katmanında her hata sabit bir `code` taşır.
- Tauri sınırında `{ code, message }` döner; ham stack trace gösterilmez.

## Veri dizini yapısı

```text
<data_dir>/
  case.sqlite
  cases/<case_id>/
    evidence/original/<stored_name>
    derived/<evidence_id>.16k.wav
    exports/<file>
```
