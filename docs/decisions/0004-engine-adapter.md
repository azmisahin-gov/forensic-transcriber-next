# ADR 0004 — Transkripsiyon motoru adaptör arkasında

## Bağlam
Varsayılan motor whisper.cpp'dir. Gelecekte alternatif motorlar gerekebilir.
İş hattı belirli bir çalıştırılabilir dosyaya bağlı kalmamalıdır.

## Karar
`EngineAdapter` trait'i tanımlanır (`id`, `probe`, `transcribe`).
`WhisperCppAdapter` varsayılan uygulamadır. İş hattı ve testler yalnızca trait'e
bağlıdır; testler sahte motor enjekte eder.

## Sonuçlar
- Motor değiştirilebilir; testler deterministik.
- Her adaptör motor sürümünü, model kimliğini ve ayarları kaydeder.

## Alternatifler
Doğrudan whisper.cpp çağrıları (test edilemez, değiştirilemez).

## Not
Alternatif motor eklemek gerçek bir ölçüm ve gerekçe gerektirir; varsayım ile
eklenmez.
