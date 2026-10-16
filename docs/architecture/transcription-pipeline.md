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
  `text` alanlarını okur.
- `-ng` yalnızca GPU istenmediğinde eklenir.
- `--help` çıktısından sürüm satırı tespit edilir (`probe`).

> **Doğrulama durumu:** Bu ortamda `whisper-cli` kurulu olmadığından gerçek uçtan
> uca transkripsiyon **çalıştırılmadı**. Argüman üretimi ve JSON ayrıştırma
> birim testleriyle doğrulanmıştır; gerçek motorla koşu doğrulanmamıştır.

## İlerleme ve iptal

- Motor stderr satırları okunur; `progress = NN%` kalıbı `[0,1]` oranına çevrilir.
- `CancelToken` (`Arc<AtomicBool>`) paylaşılır; iptal isteğinde alt süreç
  öldürülür, yarım çıktı silinir ve `EngineError::Cancelled` döner.

## Hata davranışı

| Durum | Davranış |
| --- | --- |
| Motor yok | `ENGINE_NOT_FOUND`; iş FAILED; **transkript yazılmaz** |
| Model yok | `MODEL_NOT_FOUND` |
| Girdi yok | `INPUT_NOT_FOUND` |
| FFmpeg yok ve dönüşüm gerekiyor | `MEDIA_TOOL_MISSING` |
| Segment zamanı geçersiz | Segment atlanır, sayılır, kaydedilmez |
| Aynı kanıt için tekrar koşu | `TRANSCRIPT_EXISTS`; mevcut çıktı ezilmez |

Model bulunamadığında örnek/uydurma transkript **gösterilmez**; kullanıcıya
yapılandırma eksikliği açıkça bildirilir.
