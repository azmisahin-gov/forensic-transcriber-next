# Katkı Rehberi

## Geliştirme ortamı

1. Node.js 20+, Rust stable (MSVC), Visual Studio Build Tools (C++), CMake kurun.
2. Bağımlılıkları kurun:

   ```powershell
   cargo fetch
   cd apps/desktop; npm install
   ```

## Değişiklik yapmadan önce

- `AGENTS.md` ve ilgili `docs/` belgesini okuyun.
- Büyük mimari kararlar için `docs/decisions/` altına bir ADR ekleyin.

## Göndermeden önce

```powershell
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test

cd apps/desktop
npm run lint
npm run typecheck
npm run test
npm run build
```

Tüm komutlar geçmeden değişiklik göndermeyin. Testleri "geçti" saymak yerine
gerçek çıktıyı doğrulayın.

## Test ilkeleri

- Testler sentetik veya uygun izinle anonimleştirilmiş veri kullanır.
- Gerçek kişisel kayıtlar depoya girmez.
- Harici araç gerektiren testler (FFmpeg, whisper.cpp) bu araçlar yoksa
  atlanmalı ve atlandığını açıkça bildirmelidir.

## Commit ve yayın

- Commit mesajları kısa ve açıklayıcı olmalıdır.
- `main`/`master` dalına yapılan itme yalnızca CI çalıştırır; sürüm etiketi
  atmaz.
- Sürüm yayınlama ve imzalama ayrı bir prosedürdür; bkz. `docs/security/threat-model.md`.
