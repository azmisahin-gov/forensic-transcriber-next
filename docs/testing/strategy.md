# Test Stratejisi

## Katmanlar

### Rust birim testleri
Her crate kendi modül testlerini içerir:
- `ftn-domain` — varlık kuralları, zaman biçimlendirme, durum ayrıştırma, segment
  zamanlama, düzenleme/ayırma, bütünlük mantığı.
- `ftn-integrity` — bilinen SHA-256 vektörleri, yol geçişi reddi, benzersiz ad,
  doğrulanmış kopyalama, uyuşmazlık tespiti.
- `ftn-storage` — migrasyon idempotansı, ileri şema reddi.
- `ftn-media` — ffprobe JSON ayrıştırma, çoklu ses akışı, eksik süre, geçersiz
  JSON.
- `ftn-transcription` — argüman üretimi, JSON ayrıştırma, ilerleme ayrıştırma,
  motor yok davranışı.
- `ftn-export` — SRT zaman biçimi, şema sürümü, özet sayıları, HTML kaçışlama,
  güvenli dosya adı.

### Storage entegrasyon testleri
`crates/storage-sqlite/tests/` — foreign key zorlaması, cascade silme,
`integrity_check`, şema sürümü, ayarlar.

### Uçtan uca akış
`tests/integration/tests/e2e_pipeline.rs` — vaka oluştur → içe aktar → hash →
transkripsiyon → düzenle → denetim → TXT/SRT/JSON dışa aktarma → yeniden açma →
bütünlük doğrulama. Gerçek FFmpeg kullanır, motor sahtedir.

### Dayanıklılık
`tests/integration/tests/resilience.rs` — eksik dosya, kurcalanmış kopya,
silinmiş kopya, Unicode başlık, yeniden açma.

### Frontend
- `tsc --noEmit` (strict).
- `vitest` + Testing Library: bileşen davranışı, boş durum, dışa aktarma
  düğmelerinin durumu, biçimlendirme yardımcıları.
- `eslint`.

## Çalıştırma

```powershell
cargo test
cd apps/desktop; npm run lint; npm run typecheck; npm run test; npm run build
```

## İlkeler

- Testler gerçek iş kurallarına bağlanır; kapsam için kapsam testi yazılmaz.
- Gerçek kişisel veri kullanılmaz; sentetik veya üretilmiş veri kullanılır.
- Harici araç gerektiren testler araç yoksa atlanır ve atlandığını bildirir.
- Başarısız bir test geçmiş sayılmaz.

## Boşluklar

- Gerçek `whisper-cli` ile uçtan uca koşu test edilmedi.
- Arayüz etkileşim testleri (klavye navigasyonu, oynatıcı) henüz yok.
- Yedekleme/geri yükleme ve `.ftcase` arşiv testleri henüz yok.
