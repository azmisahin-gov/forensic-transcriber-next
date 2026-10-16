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
| Rust çekirdeği (domain/integrity/storage/media/transcription/export/application) | Çalışıyor, 85 test |
| Uçtan uca akış (vaka→içe aktarma→hash→transkripsiyon→düzenleme→dışa aktarma→yeniden açma) | Çalışıyor (gerçek FFmpeg, sahte motor) |
| Tauri 2 masaüstü kabuğu | Derleniyor, açılıyor ve gerçek arayüzü çiziyor (Windows 11'de doğrulandı) |
| Arka plan transkripsiyon + ilerleme/iptal | Çalışıyor (gerçek süreç olaylarına bağlı) |
| Yerel dosya seçme penceresi | Çalışıyor (tauri-plugin-dialog) |
| Ses oynatma + zaman çizgisi | Çalışıyor (asset protokolü, gerçek `<audio>`; dalga biçimi henüz yok) |
| Motor yokken hazırlık ekranı | Çalışıyor — uygulama içinden indirme (hash doğrulamalı) |
| whisper.cpp motoru + model otomatik algılama | Çalışıyor (uygulama dizini, veri dizini, PATH) |
| İlk kurulumda motor + model indirme | Çalışıyor (pinlenmiş SHA-256 ile doğrulanır; sonrası çevrimdışı) |
| Windows yükleyicisi (NSIS + MSI) | Üretiliyor (`npm run tauri -- build`) |
| **Gerçek Türkçe transkripsiyon (uçtan uca)** | Gerçek whisper.cpp + base model ile doğrulandı (Türkçe metin üretildi) |
| Başlangıç hatası dayanıklılığı | Uygulama açılır, hata banner'ı gösterilir |
| React/TS arayüzü | typecheck + build + 24 test geçiyor |
| Arama, yedekleme, `.ftcase` arşivi, ikinci inceleme | **Henüz yok** (yol haritasında) |
| Kod imzalama | **Yok** — imzalı gibi davranılmıyor |

Ayrıntılı aşama durumu ve bilinen sınırlamalar için `docs/` altına bakın.

---

## Kurulum (son kullanıcı, teknik bilgi gerekmez)

1. `Forensic Transcriber Next_0.1.0_x64-setup.exe` (NSIS) yükleyicisini
   çalıştırın. Node.js, Rust, Cargo, Visual Studio veya komut satırı
   **gerekmez**; arayüz ve kabuk uygulamaya gömülüdür.
2. Uygulamayı açın. İlk açılışta **İlk kurulum** bölümü görünür:
   - **Motoru indir** — whisper.cpp Windows CPU sürümü indirilir, SHA-256 ile
     doğrulanır ve çıkarılır.
   - **Model indir** — çok dilli bir GGML modeli (varsayılan: base) indirilir ve
     SHA-256 ile doğrulanır.
   İnternet **yalnızca** bu indirmeler için kullanılır. Ses kayıtlarınız hiçbir
   zaman uzak bir servise gönderilmez.
3. İndirme tamamlandıktan sonra uygulama tamamen **çevrimdışı** çalışır.
4. **Yeni Vaka** → **Dosya seç ve içe aktar** → **Yazıya çevir** → transkripti
   inceleyin/düzenleyin → **Dışa aktar** (TXT/SRT/JSON/HTML).

Vaka verisi varsayılan olarak `%APPDATA%\gov.azmisahin.forensictranscribernext`
altında tutulur. `FTN_DATA_DIR` ile değiştirilebilir.

> Yükleyici **kod imzalı değildir**; Windows SmartScreen ilk açılışta uyarabilir.
> İmzalı gibi davranılmaz.

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

# Masaüstü uygulaması (geliştirme; Vite sunucusuyla birlikte)
npm run tauri dev
```

### Önemli: geliştirme mi, paketlenmiş uygulama mı?

Tauri, `debug`/`cargo build` çıktısını **geliştirme modu** sayar ve gömülü
varlıklar yerine `devUrl`'e (`http://localhost:1420`) yönlenir. Bu yüzden Vite
sunucusu çalışmıyorken tek başına `cargo build` çıktısı **boş/hatalı pencere**
açar ("localhost bağlanmayı reddetti").

Doğru kullanımlar:

- Geliştirme: `cd apps/desktop && npm run tauri dev`
- Tek başına çalışan uygulama: `cd apps/desktop && npm run tauri -- build`
  (installer istemiyorsanız `-- --no-bundle`). Bu, varlıkları `http://tauri.localhost/`
  altına gömer ve harici bir sunucu gerektirmez.

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
cargo test                     # çekirdek + entegrasyon (85 test)

cd apps/desktop
npm run lint
npm run typecheck
npm run test                   # 22 test
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
