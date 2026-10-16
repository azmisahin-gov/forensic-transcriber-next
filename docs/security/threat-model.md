# Tehdit Modeli

## Varlıklar

- Orijinal kanıt dosyaları (bütünlüğü kritik).
- SQLite vaka veritabanı (transkriptler, inceleme geçmişi, denetim kayıtları).
- Türetilmiş çalışma sesi.
- Dışa aktarılan dosyalar.

## Güven sınırları

- **Arayüz ↔ Rust.** Arayüz güvenilmez kabul edilir; tüm parametreler Rust
  tarafında doğrulanır.
- **Uygulama ↔ alt süreç.** FFmpeg/whisper-cli süreçleri argüman dizisiyle
  başlatılır, kabuk kullanılmaz.
- **Yerel kullanıcı.** Veritabanı ve dosyalar üzerinde tam yetkiye sahiptir.

## Tehditler ve korumalar

| Tehdit | Koruma | Kalan risk |
| --- | --- | --- |
| Yol geçişi (path traversal) | `safe_file_name`, `unique_file_name`, tek bileşen doğrulaması | Doğrudan dosya sistemi erişen yerel kullanıcı |
| Komut enjeksiyonu | `Command::new(bin).args([...])`, kabuk yok | — |
| Kopyalama sırasında bozulma | Geçici dosya + SHA-256 doğrulama + rename | Disk arızası |
| Sessiz veri kaybı | Orijinal salt okunur; transaction; eksik/uyuşmazlık raporlanır | — |
| Otomatik çıktının ezilmesi | `original_text` ve `transcript_revisions` | — |
| Denetim kaydının kaldırılması | — | **Yerel kullanıcı hash zincirini de değiştirebilir** |
| Ağ üzerinden veri sızıntısı | Ağ çağrısı yok; CSP kısıtlı | Gelecekte eklenecek özellikler |
| Sahte doğrulama iddiası | Kod ve belgeler gerçek durumu bildirir | — |

## Açıkça belirtilen sınırlar

- SHA-256 ve denetim kayıtları **yazılım köken/izlenebilirlik özellikleridir**;
  adli zincir veya hukuki geçerlilik **değildir**.
- Yerel bir kullanıcı veritabanına ve dosyalara tam erişebildiğinden, yerel bir
  hash zinciri mutlak değiştirilemezlik **sağlamaz**.

## İleriki aşama (henüz uygulanmadı)

Denetim olayları için hash zinciri ve dışa aktarımlarda imza desteği
değerlendirilebilir. Bu, tehdit modeli, anahtar yönetimi ve doğrulama
prosedürünün ayrıca belgelenmesini gerektirir.

## Dağıtım

- Paketleme/imzalama prosedürü henüz uygulanmadı.
- Kod imzası varsa Tauri güncelleyicisinin doğrulanmış imza mekanizması
  kullanılmalıdır.
- **İmzalama sertifikası yoksa imzalı gibi davranılmaz.**
- Sır, token veya sertifika özel anahtarı kaynak koda konmaz.
