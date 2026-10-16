# Forensic Transcriber Next

**Yerel, çevrimdışı, Türkçe öncelikli ses transkripsiyonu ve uzman inceleme
çalışma alanı.** Tauri 2 (Rust) kabuğu + React/TypeScript arayüzü + SQLite.

> Bu yazılım teknik bir transkripsiyon ve inceleme yardımcısıdır. Tek başına adli
> bir kanaat, hukuki değerlendirme, konuşmacı kimlik tespiti veya sesin
> gerçekliğine ilişkin bir belirleme oluşturmaz. Nihai değerlendirme ve
> transkriptin kullanımı, yazılımı kullanan yetkin uzmanın sorumluluğundadır.

Bu depo, `azmisahin-gov/forensic-transcriber` (Electron) projesinden **bağımsız
bir yeniden yazımdır**. Eski proje kod veya veri olarak kopyalanmamıştır; yalnızca
kavramsal referans olarak incelenmiştir.

---

## Durum (dürüst özet)

| Alan | Durum |
| --- | --- |
| Rust çekirdeği (domain/integrity/storage/media/transcription/export/application) | Çalışıyor, test edildi |
| Uçtan uca akış (vaka→içe aktarma→hash→transkripsiyon→düzenleme→dışa aktarma→yeniden açma) | Çalışıyor (gerçek FFmpeg, sahte motor) |
| Tauri 2 masaüstü kabuğu | Derleniyor ve açılıyor (Windows 11'de doğrulandı) |
| React/TS arayüzü | typecheck + build + 8 test geçiyor |
| whisper.cpp uçtan uca gerçek transkripsiyon | **Doğrulanmadı** — bu ortamda `whisper-cli` kurulu değil |
| Ses oynatıcı, dalga biçimi, arama, yedekleme, `.ftcase` arşivi, ikinci inceleme | **Henüz yok** (yol haritasında) |

Ayrıntılı aşama durumu ve bilinen sınırlamalar için `docs/` altına bakın.

---

## Mimari

```
React + TypeScript (apps/desktop/src)
        │  Tauri command surface (apps/desktop/src-tauri)
        ▼
ftn-application  (use case'ler)
        ├── ftn-domain        (varlıklar, iş kuralları; dış bağımlılık yok)
        ├── ftn-storage       (SQLite, parametreli sorgular, migrasyonlar)
        ├── ftn-media         (FFmpeg/FFprobe, güvenli süreç başlatma)
        ├── ftn-transcription (whisper.cpp adaptörü, motor arayüzü)
        └── ftn-export        (TXT/SRT/JSON/HTML)
```

- İş kurallarının tek yetkili kopyası Rust tarafındadır; arayüz ikinci bir kopya
  tutmaz.
- Veritabanına doğrudan React katmanından erişilmez.
- Ayrıntı: `docs/architecture/overview.md`.

---

## Gereksinimler

- Windows 10/11 x64.
- Geliştirme için: Node.js 20+, Rust stable (MSVC), Visual Studio Build Tools
  (C++ ile), CMake.
- Çalışma zamanında: transkripsiyon için `whisper-cli` (whisper.cpp) ve bir GGML
  modeli; ses dönüştürme için FFmpeg. İkisi de **depoda yer almaz**.

---

## Kurulum ve çalıştırma

```powershell
# Rust çalışma alanı
cargo build
cargo test

# Frontend
cd apps/desktop
npm install
npm run typecheck
npm run test
npm run build

# Masaüstü uygulaması (geliştirme)
npm run tauri dev
```

### Ortam değişkenleri

| Değişken | Anlamı |
| --- | --- |
| `FTN_DATA_DIR` | Vaka verisinin tutulduğu dizin (varsayılan: uygulama veri dizini) |
| `FTN_ENGINE_PATH` | `whisper-cli` yürütülebilir dosyasının yolu |

---

## Doğrulama komutları

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test                     # çekirdek + entegrasyon (74 test)

cd apps/desktop
npm run lint
npm run typecheck
npm run test                   # 8 test
npm run build
```

Tauri kabuğu (varsayılan derlemeye dahil değildir):

```powershell
cargo build -p forensic-transcriber-next
```

> Entegrasyon testleri gerçek `ffmpeg` kullanır; bulunamazsa ilgili testler
> atlanır ve bunu açıkça bildirir.

---

## Kapsam sınırları

Uygulama **bilinçli olarak** şunları yapmaz: konuşmacı kimlik tespiti, ses
karşılaştırma, biyometrik tanıma, deepfake/manipülasyon tespiti, duygu/niyet/yalan
analizi, suç sınıflandırması, otomatik hukuki değerlendirme. Video yalnızca ses
izinin içeriği için kabul edilir.

Otomatik çıktı ile uzman çıktısı her zaman ayrı tutulur (`AUTOMATIC`, `REVIEWED`,
`EDITED`, `VERIFIED`). Düzenleme otomatik metni asla silmez.

---

## Lisans

Uygulama kodu **AGPL-3.0-only** (`LICENSE`). Üçüncü taraf bileşenler ve modeller
kendi lisanslarına sahiptir; bkz. `THIRD_PARTY_NOTICES.md`.

`whisper.cpp`, GGML modelleri ve FFmpeg depoya dahil değildir ve ayrı lisans
koşullarına tabidir.
