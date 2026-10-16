# Üçüncü Taraf Bileşenler ve Lisanslar

Bu belge, Forensic Transcriber Next'in kullandığı ve **depoya dahil edilmeyen**
başlıca üçüncü taraf bileşenleri listeler. Her bileşenin kendi lisansı geçerlidir.
Bu liste hukuki tavsiye değildir.

## Uygulama kodu

- Forensic Transcriber Next: **AGPL-3.0-only** (bkz. `LICENSE`).

## Rust bağımlılıkları (Cargo)

| Bileşen | Lisans (yaklaşık) | Kullanım |
| --- | --- | --- |
| rusqlite (bundled SQLite) | MIT; SQLite: Public Domain | Yerel veritabanı |
| serde, serde_json | MIT / Apache-2.0 | Serileştirme |
| thiserror | MIT / Apache-2.0 | Hata tipleri |
| sha2 | MIT / Apache-2.0 | SHA-256 |
| uuid | MIT / Apache-2.0 | Kimlikler |
| tauri, tauri-build | MIT / Apache-2.0 | Masaüstü kabuğu |

Kesin lisans ve sürümler `Cargo.lock` ve ilgili crate'lerin meta verileriyle
doğrulanmalıdır. Yayın öncesi otomatik lisans denetimi yapılmalıdır.

## Arayüz bağımlılıkları (npm)

| Bileşen | Lisans (yaklaşık) | Kullanım |
| --- | --- | --- |
| react, react-dom | MIT | Arayüz |
| @tauri-apps/api, @tauri-apps/cli | MIT / Apache-2.0 | IPC ve araçlar |
| vite, vitest, typescript, eslint | MIT | Geliştirme araçları |

## Harici araçlar (depoda YOKTUR)

| Araç | Lisans | Not |
| --- | --- | --- |
| whisper.cpp | MIT | Ayrı indirilir/derlenir |
| FFmpeg | LGPL/GPL (derlemeye göre) | Ayrı kurulur; LGPL derlemeler tercih edilir |
| GGML Whisper modelleri | Model başına değişir | Ayrı indirilir |

Bu araçlar depoya dahil edilmez; kullanıcı tarafından sağlanır veya kurulur.
Dağıtım sırasında her birinin lisans koşulları ayrıca incelenmelidir.
