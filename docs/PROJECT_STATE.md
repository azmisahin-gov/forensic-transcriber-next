# Proje Durumu

Son güncelleme: CI düzeltmesi + gerçek transkripsiyon yolu sertleştirmesi +
arka plan ilerleme/iptal. Commit: `f7b537c` sonrası.

Bu belge, her işlevi **uygulanmış / kısmi / uygulanmamış** olarak açıkça ayırır.
Test edilmemiş hiçbir özellik tamamlanmış gösterilmez.

## CI düzeltmesi

İlk CI koşusu (`38965841270`) Rust işinde başarısızdı. Kök neden: iş akışı
`cargo clippy --workspace --default-members ...` ve `cargo test --workspace
--default-members ...` komutlarını kullanıyordu; **cargo'da `--default-members`
diye bir argüman yoktur** ve `--workspace` ile birlikte de geçersizdir. Komut
daha clippy çalışmadan hata veriyordu (yerelde birebir doğrulandı).

Düzeltme: `--workspace --exclude forensic-transcriber-next` kullanıldı. Tauri
kabuğu Linux'ta `webkit2gtk` sistem kütüphaneleri gerektirdiğinden Linux
işlerinden hariç tutuldu; ayrı Windows işinde derleniyor.

## İşlev durumları

| İşlev | Durum | Not |
| --- | --- | --- |
| Rust çekirdeği (domain/integrity/storage/media/transcription/export/application) | **Uygulanmış** | 85 test |
| Uçtan uca akış (vaka→içe aktarma→hash→transkripsiyon→düzenleme→dışa aktarma→yeniden açma) | **Uygulanmış** | Gerçek FFmpeg + sahte motor |
| SHA-256 bütünlük doğrulama | **Uygulanmış** | Kurcalama/eksik testleri |
| Tauri kabuğu + React arayüzü | **Uygulanmış** | Derlenip açılıyor, arayüz render ediliyor (Win11, CDP ile doğrulandı) |
| whisper.cpp CLI sözleşmesi (argüman, çıktı yolu, JSON ayrıştırma) | **Uygulanmış** | Birim + gated gerçek test |
| **Gerçek transkripsiyon motoru doğrulaması (uçtan uca)** | **Uygulanmış** | Gerçek whisper.cpp v1.9.2 + ggml-base ile uygulama içinden Türkçe metin üretildi (CDP ile doğrulandı) |
| İlerleme bildirimi (arka plan) | **Uygulanmış** | `transcription:progress` Tauri event'i, gerçek süreç stderr'ine bağlı |
| İptal | **Uygulanmış** | `cancel_transcription` + `CancelToken`; alt süreç öldürülür, yarım çıktı temizlenir |
| İş durumu sorgulama | **Uygulanmış** | `job_status`, kalıcı `transcription_jobs` |
| Uzun iş sırasında arayüzün bloke olmaması | **Uygulanmış** | begin/execute/complete ayrımı; kilit yalnızca kısa DB fazlarında |
| Yerel dosya seçme penceresi | **Uygulanmış** | `tauri-plugin-dialog` (`dialog:allow-open`) |
| Ses oynatma + zaman çizgisi | **Uygulanmış (temel)** | Asset protokolü ile gerçek `<audio>`; oynat/duraklat, ±5 sn, kaydırma. Dalga biçimi yok |
| Motor + model otomatik algılama | **Uygulanmış** | Env → veri dizini → uygulama dizini → PATH |
| İlk kurulumda motor + model indirme | **Uygulanmış** | `prepare_engine` / `download_model`; pinlenmiş SHA-256; ilerleme + iptal; sonrası çevrimdışı |
| Windows yükleyicisi (NSIS + MSI) | **Uygulanmış** | `npm run tauri -- build` → setup 3.41 MiB, MSI 5.87 MiB |
| Başlangıç hatası dayanıklılığı | **Uygulanmış** | Uygulama açılır, `startup_status` + banner; ErrorBoundary |
| Vaka arama | **Uygulanmadı** | |
| Vaka yedekleme ve geri yükleme | **Uygulanmadı** | |
| `.ftcase` arşivi | **Uygulanmadı** | |
| İkinci inceleme | **Uygulanmadı** | |
| Dalga biçimi görünümü | **Uygulanmadı** | |
| Kod imzalama | **Uygulanmadı** | İmzalı gibi davranılmıyor |
| Türkçe doğruluk ölçümü (WER/CER) | **Uygulanmadı** | Doğrulanmış referans veri kümesi yok; sayı iddiası yok |

## Bu ortamda doğrulanan komut sonuçları

| Komut | Sonuç |
| --- | --- |
| `cargo fmt --all --check` | Geçti |
| `cargo clippy --workspace --all-targets -- -D warnings` (Tauri dahil) | Geçti |
| `cargo test --workspace --exclude forensic-transcriber-next` | Geçti — 85 test (whisper_real: SKIP) |
| `npm run lint` / `typecheck` / `test` (24) / `build` | Geçti |
| `npm run tauri -- build` | Geçti — NSIS + MSI installer üretildi |
| Gerçek uygulamada Türkçe transkripsiyon (whisper.cpp + base) | CDP ile doğrulandı — Türkçe metin üretildi |
| Gerçek uygulamada TXT/SRT/JSON dışa aktarma | CDP ile doğrulandı (SHA-256'lı çıktılar) |
| Gerçek uygulamada kalıcılık + düzenleme | CDP ile doğrulandı (yeniden açınca transkript yüklendi; otomatik metin korundu) |
| Gerçek uygulamada motor indirme + hash doğrulama | CDP ile doğrulandı |
| Temiz Windows makinede kurulum | **Çalıştırılmadı** — bu ortamda temiz VM yok |

## Bilinen sorunlar ve boşluklar

1. Temiz bir Windows makinede installer kurulumu bu ortamda doğrulanmadı.
2. Yükleyici kod imzalı değil; SmartScreen uyarabilir (imzalı gibi davranılmaz).
3. Model varsayılanı `base` (hızlı, sınırlı doğruluk). Daha iyi Türkçe için daha
   büyük model veri dizinine bırakılabilir (otomatik algılanır).
4. Dalga biçimi ve transkript satırından ses konumuna gitme henüz yok.
5. `npm audit` dev bağımlılıklarında uyarılar var; incelenmesi gerekir.

## Öncelik sırası

veri bütünlüğü → doğru veri modeli → güvenli mimari → çalışan transkripsiyon hattı
→ uzman kullanıcı deneyimi → performans ve yayın kalitesi.
