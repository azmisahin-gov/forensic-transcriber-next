# AGENTS.md

Bu depo üzerinde çalışan geliştiriciler ve otomatik ajanlar için kalıcı
yönergeler. Önce bunu okuyun.

## Bu depo nedir

Tauri 2 (Rust) masaüstü kabuğu, React/TypeScript arayüzü ve SQLite veri modeli
üzerine kurulu, yerel ve çevrimdışı çalışan bir ses transkripsiyonu ve uzman
inceleme uygulaması.

## Değişmez kurallar

1. **Kapsam.** Konuşmacı kimlik tespiti, ses karşılaştırma, biyometrik tanıma,
   deepfake/manipülasyon tespiti, duygu/niyet/yalan analizi, suç sınıflandırması
   veya hukuki yorum yok. Video yalnızca ses izi için kabul edilir.
2. **Makine ≠ uzman.** `AUTOMATIC` çıktı asla uzman çıktısına dönüştürülmez.
   Dört durum korunur; otomatik metin düzenlemede daima saklanır.
3. **Hukuki iddia yok.** Hash ve kayıtlar "yazılım köken/izlenebilirlik
   özellikleri" olarak adlandırılır; adli zincir veya hukuki geçerlilik iddiası
   kurulmaz.
4. **Çevrimdışı öncelikli.** Telemetri, analitik, giriş, hesap, bulut veya ses
   yükleme yok.
5. **Güvenlik.** Arayüz güven sınırı değildir; her parametre Rust tarafında
   doğrulanır. Alt süreçler argüman dizisiyle ve kabuk olmadan başlatılır.
6. **Yanlış doğrulama yok.** Gerçek bir komut üretmedikçe "test edildi",
   "çalışıyor" veya "üretime hazır" yazılmaz. Gerçek durum raporlanır.
7. **Model ve ikili dosya git'e girmez.** `*.bin`, `*.ggml`, `models/`,
   `vendor/` yok sayılır; testlerde yalnızca sentetik/üretilmiş veri kullanılır.

## Katmanlar

- `crates/domain` — varlıklar ve iş kuralları; dış bağımlılığı yoktur.
- `crates/application` — kullanıcı use case'leri; iş kurallarının tek kopyası.
- `crates/storage-sqlite` — SQLite; tüm SQL burada, parametreli.
- `crates/media` — FFmpeg/FFprobe; güvenli `Command` ile.
- `crates/transcription` — `EngineAdapter` arayüzü + whisper.cpp adaptörü.
- `crates/export` — TXT/SRT/JSON/HTML.
- `apps/desktop/src-tauri` — Tauri komut yüzeyi (DTO'lar camelCase).
- `apps/desktop/src` — React arayüzü; iş kuralı içermez.

## Kurallar

- Aynı iş kuralını hem TypeScript hem Rust içinde uygulama.
- `any`, gelişigüzel `unwrap()`, kontrolsüz `panic!` ve sessiz hata yutmayı önle.
- Rust'ta `Result`/`Error` (thiserror); arayüze güvenli `{ code, message }` döner.
- Ham stack trace'i kullanıcıya gösterme; loglara transkript metni yazma.
- Yorumlar *neden*i açıklar, *ne*yi değil. Dosyaları küçük tut.
- Kullanılmayan bağımlılık ve ölü kod bırakma.

## Doğrulama komutları

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test

cd apps/desktop
npm run lint
npm run typecheck
npm run test
npm run build
```

Tauri kabuğu: `cargo build -p forensic-transcriber-next`.

## Faz iş akışı

Her önemli değişiklikte ilgili `docs/` belgesini ve `docs/decisions/` altındaki
ADR'leri güncelle. İş yarıda kaldıysa `COMPLETED`, `IN_PROGRESS`, `BLOCKER`,
`NEXT_EXACT_ACTION` ve `VERIFICATION_STATUS` alanlarını dürüstçe kaydet; gerçek
bir test çalıştırmadan hiçbir şeyi tamamlandı olarak işaretleme.
