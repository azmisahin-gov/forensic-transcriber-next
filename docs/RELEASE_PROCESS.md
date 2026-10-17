# Sürüm Süreci

İki ayrı süreç vardır; normal `main` push'ları otomatik sürüm yayımlamaz.

## CI (`.github/workflows/ci.yml`)

Her `main` push'u ve pull request için:

- Rust: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.
- Frontend: `npm ci`, sürüm tutarlılığı, lint, typecheck, test, build.
- Windows Tauri derleme kontrolü.

## Kontrollü sürüm (`.github/workflows/release-version.yml`)

`workflow_dispatch` ile `patch` / `minor` / `major` seçilir. Akış:

1. Güncel `main` HEAD'inin **başarılı CI'dan geçmiş** olduğunu doğrular.
2. Yeni sürümü hesaplar ve `scripts/bump-version.mjs` ile **tüm** sürüm
   alanlarını günceller (`Cargo.toml`, `tauri.conf.json`, `package.json`,
   `package-lock.json`).
3. `scripts/check-versions.mjs` ile tutarlılığı doğrular.
4. `chore(release): vX.Y.Z` commit'i ve açıklamalı `vX.Y.Z` etiketi oluşturur,
   `main`'e ve etikete push eder.
5. `.github/workflows/release.yml`'i `workflow_call` ile çağırır.

## Build ve yayın (`release.yml`, reusable)

- Etikete göre checkout eder, sürüm/tag uyumunu doğrular.
- `TAURI_SIGNING_PRIVATE_KEY` ile **imzalı updater artifact'i** üretir.
- Sabit adlara kopyalar ve `latest.json` üretir (gerçek sürüm, gerçek asset
  URL'i, gerçek `.sig` içeriği).
- `SHA256SUMS.txt` üretir.
- `latest.json`'u yayımlamadan önce doğrular.
- GitHub Release'i ve asset'leri yayımlar.

### İlk sürüm (v0.1.0)

İlk sürüm için `release-version` kullanılmaz (0.1.0'ı 0.1.1 yapardı). Bunun
yerine:

1. `main`'de sürümler zaten `0.1.0` olmalı.
2. `git tag -a v0.1.0 -m ...` oluşturulur ve push edilir.
3. `release.yml` **workflow_dispatch** ile `version=0.1.0`, `tag=v0.1.0` olarak
   tetiklenir.

## Sonraki sürüm (kısa)

Actions → **Release (version)** → Run workflow → `patch|minor|major` seç.

## Gerekli secret

- `TAURI_SIGNING_PRIVATE_KEY` (zorunlu) — Tauri updater özel anahtarı.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (anahtar parolasızsa boş).

**Uyarı:** Özel anahtar kaybolursa mevcut kullanıcı kurulumları güncellenemez.
Anahtarın güvenli bir yedeğini saklayın. Özel anahtar repoya **konmaz**.

## Sürümün tutarlı olması gereken yerler

`tauri.conf.json`, Cargo workspace, `package.json`/lock, git etiketi, release
başlığı, `latest.json`, site. `scripts/check-versions.mjs` bunları CI'da denetler.
