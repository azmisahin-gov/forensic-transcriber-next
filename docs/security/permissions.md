# Tauri Yetkileri ve Süreç Sınırları

## Yetenek (capability)

`apps/desktop/src-tauri/capabilities/default.json` yalnızca şunu verir:

```json
{ "permissions": ["core:default"] }
```

- Dosya sistemi (`fs`), kabuk (`shell`), diyalog (`dialog`) ve HTTP (`http`)
  eklentileri **kullanılmaz** ve yetkisi verilmez.
- Renderer genel amaçlı dosya sistemi veya süreç çalıştırma yetkisine sahip
  değildir.

## Komut yüzeyi

İzin verilen komutlar yalnızca aşağıdakilerdir:

`list_cases`, `create_case`, `get_case`, `list_evidence`, `import_evidence`,
`verify_evidence`, `transcribe`, `transcript_for_evidence`, `list_segments`,
`edit_segment`, `export_transcript`, `engine_probe`.

Her komut parametreleri doğrulanmak üzere uygulama katmanına iletir; komutun
kendisi güvenlik doğrulaması yapmaz, UI de yapamaz.

## CSP

`tauri.conf.json` içinde kısıtlı bir içerik güvenliği politikası ayarlıdır:

```
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
img-src 'self' asset: data:; connect-src 'self' ipc: http://ipc.localhost;
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

- Uygulama başlangıcında hiçbir ağ çağrısı yapılmaz.
- Telemetri varsayılan olarak kapalıdır ve hiçbir telemetri toplanmaz.
- Gelecekte model indirme eklenirse ayrı, açık kullanıcı eylemi ve doğrulanabilir
  bir model manifesti gerektirir.
