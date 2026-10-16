# Tauri Yetkileri ve Süreç Sınırları

## Yetenek (capability)

`apps/desktop/src-tauri/capabilities/default.json` yalnızca şunu verir:

```json
{ "permissions": ["core:default", "dialog:allow-open"] }
```

- `dialog:allow-open` yalnızca **açma** penceresine izin verir; kaydetme,
  mesaj veya onay diyalogları verilmez.
- Dosya sistemi (`fs`), kabuk (`shell`) ve HTTP (`http`) eklentileri
  **kullanılmaz** ve yetkisi verilmez.
- Renderer genel amaçlı dosya sistemi veya süreç çalıştırma yetkisine sahip
  değildir.

## Asset protokolü (ses oynatma)

- `assetProtocol.enable = true` ve başlangıçta **boş kapsam** (scope) ile açılır.
- Çalışma zamanında yalnızca yapılandırılmış veri dizini
  (`app.asset_protocol_scope().allow_directory(data_dir, true)`) eklenir.
- Yol her zaman backend'den gelir; kullanıcı yolu renderer'da URL'e
  dönüştürülmez.
- Böylece yalnızca yönetilen kanıt kopyaları akıtılabilir; keyfi dosyalar
  açılamaz.

## Komut yüzeyi

İzin verilen komutlar yalnızca aşağıdakilerdir:

`startup_status`, `list_cases`, `create_case`, `get_case`, `list_evidence`,
`import_evidence`, `verify_evidence`, `transcribe`, `cancel_transcription`,
`job_status`, `transcript_for_evidence`, `list_segments`, `edit_segment`,
`export_transcript`, `engine_probe`.

Her komut parametreleri doğrulanmak üzere uygulama katmanına iletir; komutun
kendisi güvenlik doğrulaması yapmaz, UI de yapamaz.

## CSP

`tauri.conf.json` içinde kısıtlı bir içerik güvenliği politikası ayarlıdır:

```
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
img-src 'self' asset: http://asset.localhost data:;
media-src 'self' asset: http://asset.localhost blob:;
connect-src 'self' ipc: http://ipc.localhost;
object-src 'none'; base-uri 'self'; frame-ancestors 'none'
```

Uzak içerik yüklenmez; uzaktan çalışan kod yoktur.

## Alt süreçler

- FFmpeg/FFprobe/whisper-cli yalnızca `ftn-media` ve `ftn-transcription` içinden,
  `Command::new(path).args(&[...])` ile başlatılır.
- Çalışma dizini ve dosya yolları doğrulanır.
- Çıktı boyutu sınırlanır (stderr hata metni 2000 karaktere kısılır).
- Hassas içerik (transkript metni, ses) loglara yazılmaz.

## Ağ

- Uygulama başlangıcında hiçbir ağ çağrısı yapılmaz; telemetri yoktur.
- İlk kurulumdaki **motor ve model indirmeleri** yalnızca kullanıcının açık
  eylemiyle başlar. Kaynaklar sabittir:
  - Motor: `github.com/ggml-org/whisper.cpp` resmî `v1.9.2` sürüm arşivi
    (`whisper-bin-x64.zip`, pinlenmiş SHA-256).
  - Model: `huggingface.co/ggerganov/whisper.cpp` resmî GGML dosyası (pinlenmiş
    SHA-256).
- İndirilen dosyanın SHA-256'sı eşleşmezse dosya reddedilir ve kullanılmaz.
- Model bir kez doğrulandıktan sonra transkripsiyon çevrimdışı çalışır.
- Ses kayıtları ve transkriptler hiçbir koşulda ağa gönderilmez.
