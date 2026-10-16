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
| Rust çekirdeği (domain/integrity/storage/media/transcription/export/application) | **Uygulanmış** | 83 test |
| Uçtan uca akış (vaka→içe aktarma→hash→transkripsiyon→düzenleme→dışa aktarma→yeniden açma) | **Uygulanmış** | Gerçek FFmpeg + sahte motor |
| SHA-256 bütünlük doğrulama | **Uygulanmış** | Kurcalama/eksik testleri |
| Tauri kabuğu + React arayüzü | **Uygulanmış** | Derlenip açılıyor (Win11) |
| whisper.cpp CLI sözleşmesi (argüman, çıktı yolu, JSON ayrıştırma) | **Uygulanmış** | Birim + gated gerçek test |
| **Gerçek transkripsiyon motoru doğrulaması (uçtan uca)** | **Kısmi** | Kod hazır; bu makinede `whisper-cli` + model yok, gerçek koşu yapılmadı. `whisper_real.rs` env ile koşar. |
| İlerleme bildirimi (arka plan) | **Uygulanmış** | `transcription:progress` Tauri event'i, gerçek süreç stderr'ine bağlı |
| İptal | **Uygulanmış** | `cancel_transcription` + `CancelToken`; alt süreç öldürülür, yarım çıktı temizlenir |
| İş durumu sorgulama | **Uygulanmış** | `job_status`, kalıcı `transcription_jobs` |
| Uzun iş sırasında arayüzün bloke olmaması | **Uygulanmış** | begin/execute/complete ayrımı; kilit yalnızca kısa DB fazlarında |
| Dosya seçme penceresi | **Uygulanmadı** | İçe aktarma yol metniyle (bilinçli; en az yetki) |
| Ses oynatma ve dalga biçimi | **Uygulanmadı** | |
| Vaka arama | **Uygulanmadı** | |
| Vaka yedekleme ve geri yükleme | **Uygulanmadı** | |
| `.ftcase` arşivi | **Uygulanmadı** | |
| İkinci inceleme | **Uygulanmadı** | |
| Paketleme ve kod imzalama | **Uygulanmadı** | İmzalı gibi davranılmıyor |
| Türkçe transkripsiyon doğruluk testi | **Uygulanmadı** | Doğrulanmış referans veri kümesi yok; doğruluk iddiası yok |

## Bu ortamda doğrulanan komut sonuçları

| Komut | Sonuç |
| --- | --- |
| `cargo fmt --all --check` | Geçti |
| `cargo clippy --workspace --exclude forensic-transcriber-next --all-targets -- -D warnings` (CI ile aynı) | Geçti |
| `cargo clippy --workspace --all-targets -- -D warnings` (Tauri dahil, yerel) | Geçti |
| `cargo test --workspace --exclude forensic-transcriber-next` | Geçti — 83 test (whisper_real: SKIP) |
| `cargo build -p forensic-transcriber-next` | Geçti |
| `npm run lint` / `typecheck` / `test` (11) / `build` | Geçti |
| Gerçek `whisper-cli` entegrasyonu | **Çalıştırılmadı** — binary/model yok |

## Bilinen sorunlar ve boşluklar

1. Gerçek whisper.cpp koşusu bu ortamda doğrulanmadı (yukarıda).
2. Sahte motor testi gerçek motor testi olarak sunulmaz.
3. Arayüz etkileşim testleri (oynatıcı, klavye) henüz yok.
4. `npm audit` dev bağımlılıklarında uyarılar var; incelenmesi gerekir.
5. Yerel git commit'i bu turda yapılacak (istenen kapsamda).

## Öncelik sırası

veri bütünlüğü → doğru veri modeli → güvenli mimari → çalışan transkripsiyon hattı
→ uzman kullanıcı deneyimi → performans ve yayın kalitesi.
