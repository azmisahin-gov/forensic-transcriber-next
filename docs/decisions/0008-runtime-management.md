# ADR 0008 — Motor ve modelin uygulama içinden yönetilmesi

## Bağlam
Son kullanıcıdan Node.js, Rust, whisper.cpp veya model dosyası yolu istemek
kabul edilemez. Aynı zamanda whisper.cpp ikilileri ve GGML modelleri (yüzlerce
MB) depoya veya installer'a gömülmemelidir (lisans + boyut + güncellenebilirlik).

## Karar
- Motor ve model **önce otomatik algılanır**: `FTN_ENGINE_PATH` → veri dizini →
  uygulama dizini (dev `runtime/`) → `PATH`; model için veri dizini `models/`
  ve dev `runtime/models/`.
- Eksikse **ilk kurulum ekranı** `prepare_engine` / `download_model` komutlarıyla
  indirir. Kaynaklar sabit ve resmîdir; her indirme **pinlenmiş SHA-256** ile
  doğrulanır. İndirme ilerlemesi event ile bildirilir ve iptal edilebilir.
- Motor arşivi (`whisper-bin-x64.zip`, v1.9.2) indirilir, doğrulanır, yalnızca
  `whisper-cli.exe` ve `*.dll` dosyaları veri dizinine çıkarılır.
- Model, veri dizini `models/` altına indirilir; bir kez doğrulandıktan sonra
  transkripsiyon **çevrimdışı** çalışır.
- İnterner yalnızca bu açık kullanıcı eylemleri için kullanılır; ses/transkript
  asla gönderilmez.

## Sonuçlar
- Installer küçük kalır (NSIS ≈ 3.4 MiB); ağır varlıklar isteğe bağlı indirilir.
- Temiz makinede uygulama, geliştirme araçları olmadan kurulup çalışır.
- Hash uyuşmazlığında dosya reddedilir.
- Model değiştirmek isteyen kullanıcı `models/` dizinine dosya bırakabilir;
  en büyük `.bin` otomatik seçilir.

## Alternatifler
- Motor+modeli installer'a gömmek: installer ~150+ MB olur, her güncellemede
  yeniden indirilir, lisans/dağıtım karmaşıklığı artar.
- Kullanıcıdan yol istemek: hedef kullanıcı için kabul edilemez.
