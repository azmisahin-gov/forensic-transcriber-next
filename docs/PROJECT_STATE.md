# Proje Durumu

Son güncelleme: Faz 1–2 tamamlandı, Faz 3–6 kısmi.

## Tamamlanma ölçütleri (dürüst durum)

| Ölçüt | Durum | Kanıt / Not |
| --- | --- | --- |
| Yeni proje eski depodan bağımsız | ✅ | Ayrı dizin + yeni yerel git deposu |
| Proje yapısı ve mimari belgeler mevcut | ✅ | `docs/architecture/` |
| Temel masaüstü uygulaması açılıyor | ✅ | Tauri kabuğu derlendi, çalıştırıldı, DB oluştu (Windows 11) |
| Vaka oluşturma ve yeniden açma | ✅ | `e2e_pipeline` testi |
| Ses içe aktarma | ✅ | Gerçek FFmpeg ile E2E testi |
| Orijinal ve türetilmiş medya ayrılmış | ✅ | `evidence/original/`, `derived/` |
| SHA-256 doğrulaması | ✅ | `ftn-integrity` + E2E + kurcalama testi |
| SQLite kayıtları ve migrasyonlar | ✅ | `ftn-storage` testleri, `user_version` |
| Transkripsiyon servisi gerçek motorla entegre | ⚠️ Kısmi | Adaptör + ayrıştırıcı test edildi; gerçek `whisper-cli` bu ortamda yok |
| Model yokluğunda dürüst hata davranışı | ✅ | `missing_engine_fails_honestly` testi |
| Transkript düzenleme ve sürüm geçmişi | ✅ | `original_text`, `transcript_revisions` |
| TXT, SRT, JSON dışa aktarma | ✅ | `ftn-export` + E2E (HTML de var) |
| Veri kaybına karşı dayanıklılık testleri | ✅ | `resilience.rs`, storage entegrasyonu |
| Güvenli süreç + Tauri yetki sınırları | ✅ | `Command::args`, `core:default`, kısıtlı CSP |
| Derleme ve otomatik test sonuçları doğrulanmış | ✅ | Aşağıdaki bölüme bakın |
| Kurulum/geliştirme talimatları | ✅ | `README.md`, `CONTRIBUTING.md` |
| Çözülememiş sorunlar listelenmiş | ✅ | Aşağıda |

## Doğrulanan komut sonuçları (bu ortamda)

| Komut | Sonuç |
| --- | --- |
| `cargo test --workspace` | Geçti — 74 test (çekirdek + entegrasyon) |
| `cargo fmt --all --check` | Geçti (0 fark) |
| `cargo clippy --workspace --all-targets -- -D warnings` | Geçti (0 uyarı) |
| `npm run lint` | Geçti |
| `npm run typecheck` (strict) | Geçti |
| `npm run test` | Geçti — 8 test |
| `npm run build` | Geçti (Vite üretim derlemesi) |
| `cargo build -p forensic-transcriber-next` | Geçti (14.8 MB exe) |
| Masaüstü smoke (exe başlatma) | Geçti — pencere açıldı, `case.sqlite` + migrasyonlar oluştu |

## Bilinen sorunlar / eksikler

1. **whisper.cpp uçtan uca doğrulanmadı.** `whisper-cli` bu ortamda kurulu
   değil. Argüman üretimi ve JSON ayrıştırma birim testleriyle doğrulandı; gerçek
   koşu test edilmedi.
2. **Ses oynatıcı, dalga biçimi, klavye kısayolları** henüz yok (Faz 4).
3. **Vaka arşivi (`.ftcase`), yedekleme/geri yükleme** henüz yok (Faz 3).
4. **Arama, ikinci inceleme onayı** henüz yok.
5. **Dosya seçici** yok; içe aktarma yol metniyle yapılıyor (Tauri diyalog
   eklentisi bilinçli olarak eklenmedi; en az yetki ilkesi).
6. **Paketleme/imzalama** yapılmadı; imzalı gibi davranılmıyor.
7. **npm audit** dev bağımlılıklarında uyarılar bildiriyor; incelenmesi gerekir.
8. **Gerçek Türkçe doğruluk ölçümü** yapılmadı; doğruluk iddiası yok.
9. **Yerel ilk git commit'i yapılmadı** (sistem kuralı gereği; istenirse
   yapılabilir).

## Öncelik sırası

veri bütünlüğü → doğru veri modeli → güvenli mimari → çalışan transkripsiyon hattı
→ uzman kullanıcı deneyimi → performans ve yayın kalitesi.
