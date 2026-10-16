# ADR 0007 — Uzun transkripsiyon çalışmasını kilitten ayırma ve ilerleme olayları

## Bağlam
İlk uygulamada `start_transcription` tüm işi tek çağrıda, workspace mutex'ini
tutarak yapıyordu. Uzun bir motor koşusu sırasında bu kilit, vaka listesi gibi
diğer komutları bloke ediyordu. İlerleme geri çağrısı boştu ve iptal token'ı
arayüzden değiştirilemiyordu.

## Karar
Transkripsiyonu üç faza ayır:

1. `begin_transcription` — kısa, kilitli: doğrulama, RUNNING job kalıcı, plan.
2. `execute_transcription` — uzun, **kilitsiz**: FFmpeg dönüşümü + motor.
3. `complete_transcription` / `fail_transcription` — kısa, kilitli: kalıcılaştırma.

Tauri katmanı:
- İşi `spawn_blocking` ile arka planda çalıştırır.
- `transcription:progress` ve `transcription:stage` event'lerini yayar.
- `cancel_transcription(jobId)` ile `CancelToken`'ı işaretler.
- `job_status(jobId)` ile kalıcı durumu döndürür.

## Sonuçlar
- Uzun iş sırasında arayüz ve diğer komutlar yanıt verir.
- İlerleme ve iptal gerçek süreç yaşam döngüsüne bağlıdır; sahte gösterge yok.
- `TranscriptionPlan` domain dışı bir taşıyıcıdır; `Arc<dyn EngineAdapter>`
  içerdiği için `Debug` elle uygulanır.

## Alternatifler
Kilidi tutup arayüzü yalnızca asenkron komutla serbest bırakmak (diğer ekranları
yine bloke eder). Motoru ayrı bir süreç/dosya protokolüne taşımak (bu ölçek için
gereğinden karmaşık).
