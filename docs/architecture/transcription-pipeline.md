# Transkripsiyon İş Hattı

## Genel akış

```
1. Kaydı doğrula (dosya var mı, desteklenen biçim mi)
2. Orijinali koru (salt okunur kanıt kopyası)
3. Çalışma sesini üret (16 kHz mono PCM WAV) — FFmpeg
4. Transkripsiyon işini oluştur ve kalıcılaştır (QUEUED → RUNNING)
5. Motoru güvenli biçimde başlat (whisper-cli, argüman dizisi)
6. İlerleme ve iptal durumunu yönet
7. Motor çıktısını doğrula (JSON ayrıştırma)
8. Geçerli segmentleri normalize et ve kaydet (transaction)
9. Tamamlanma / hata durumunu kalıcılaştır
10. İnceleme ekranında göster
```

## Motor soyutlaması

```text
TranscriptionService
    └── EngineAdapter (trait)
            └── WhisperCppAdapter   (varsayılan)
            └── (gelecekte) alternatif adaptör
```

`EngineAdapter` üç şey tanımlar: `id()`, `probe()` ve `transcribe()`. İş hattı
hiçbir yerde belirli bir çalıştırılabilir dosyaya bağlı değildir; testler sahte
bir adaptör enjekte eder.

## whisper.cpp komut sözleşmesi

`WhisperCppAdapter::build_args` şu argümanları üretir (dizi olarak, kabuk yok):

```
-m <model> -f <input> -l <lang> -oj -of <output_base> -nt [-t <threads>] [-ng]
```

- `-oj` / `-of` JSON çıktısı; ayrıştırıcı `transcription[].offsets.{from,to}` ve
  `text` alanlarını, ayrıca varsa `model` ve `systeminfo` alanlarını okur.
- whisper.cpp `-of <base>` dosyayı `<base>.json` olarak yazar. Bu nedenle JSON
  yolu, taban adına `.json` **eklenerek** hesaplanır. Önceki hatalı
  `PathBuf::with_extension("json")` çağrısı noktalı tabanları bozup motorun
  yazdığı dosyayı bulamıyordu.
- Tüm motor çıktıları, her koşu için oluşturulan **özel geçici dizine** yazılır;
  başarı, hata veya iptal durumunda bu dizin her zaman silinir.
- `-ng` yalnızca GPU istenmediğinde eklenir.
- `--help` çıktısından sürüm satırı tespit edilir (`probe`).

> **Doğrulama durumu:** Bu makinede `whisper-cli` ve model bulunmadığından gerçek
> uçtan uca transkripsiyon **çalıştırılmadı**. Argüman üretimi, çıktı yolu ve JSON
> ayrıştırma birim testleriyle doğrulanmıştır. `tests/integration/tests/
> whisper_real.rs` gerçek koşuyu `FT_TEST_WHISPER_CLI` ve `FT_TEST_MODEL` env
> değişkenleri verildiğinde çalıştırır; aksi halde SKIP eder.

## Alt süreç akışı ve bloke olma riski

- `stdin` null; `stdout` null edilir (JSON dosyaya gider; boru dolması riski yok).
- `stderr` ayrı bir iş parçacığında okunur; ayrıştırılan ilerleme `mpsc`
  kanalıyla ana döngüye aktarılır. Böylece alt süreç dolu bir boru üzerinde bloke
  olamaz; ana döngü `try_wait` ile 100 ms aralıklarla ilerlemeyi işler ve iptali
  kontrol eder.
- İptal isteğinde alt süreç öldürülür, geçici dizin silinir ve
  `EngineError::Cancelled` döner.

## Kilit ve arka plan (begin / execute / complete)

Transkripsiyon üç faza bölünmüştür; workspace mutex'i yalnızca kısa fazlarda
tutulur:

```text
begin_transcription   (kısa, kilitli)   → plan + RUNNING job kalıcı
execute_transcription (uzun, kilitsiz)  → FFmpeg + motor
complete / fail       (kısa, kilitli)   → transkript/segment/revizyon/audit
```

Böylece uzun bir motor koşusu sırasında vaka listesi ve diğer komutlar çalışmaya
devam eder.

## İlerleme ve iptal (arayüz)

- Rust, `transcription:progress` (`{ jobId, fraction }`) ve `transcription:stage`
  (`{ jobId, stage }`) Tauri event'lerini yayar.
- `cancel_transcription(jobId)` ilgili `CancelToken`'ı işaretler.
- `job_status(jobId)` kalıcı iş durumunu döndürür.
- Arayüz ilerleme çubuğunu, iptal düğmesini ve `RUNNING/SUCCEEDED/FAILED/
  CANCELLED` durumlarını bu gerçek olaylara bağlar; sahte gösterge yoktur.

## Hata davranışı

| Durum | Davranış |
| --- | --- |
| Motor yok | `ENGINE_NOT_FOUND`; iş FAILED; **transkript yazılmaz** |
| Kullanıcı iptali | `CANCELLED`; iş CANCELLED; transkript yazılmaz |
| Model yok | `MODEL_NOT_FOUND` |
| Girdi yok | `INPUT_NOT_FOUND` |
| FFmpeg yok ve dönüşüm gerekiyor | `MEDIA_TOOL_MISSING` |
| Segment zamanı geçersiz | Segment atlanır, sayılır, kaydedilmez |
| Aynı kanıt için tekrar koşu | `TRANSCRIPT_EXISTS`; mevcut çıktı ezilmez |

Model bulunamadığında örnek/uydurma transkript **gösterilmez**; kullanıcıya
yapılandırma eksikliği açıkça bildirilir.
