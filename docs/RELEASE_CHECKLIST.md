# Sürüm Kontrol Listesi

Yayından önce:

- [ ] `main` temiz ve `origin/main` ile senkron.
- [ ] CI `main` HEAD'inde **başarılı** (release-version bunu zorunlu kılar).
- [ ] Tüm sürüm alanları tutarlı (`node scripts/check-versions.mjs`).
- [ ] Yerel: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`,
      frontend lint/typecheck/test/build geçti.
- [ ] `TAURI_SIGNING_PRIVATE_KEY` secret'ı ayarlı; özel anahtarın yedeği var.
- [ ] `tauri.conf.json` updater `pubkey` ile secret eşleşiyor.

Yayın sırasında:

- [ ] Yeni sürüm commit'i ve `vX.Y.Z` etiketi oluştu.
- [ ] Updater artifact'i ve `.sig` üretildi.
- [ ] `latest.json` doğru sürüm, URL ve **gerçek imza** içeriyor.
- [ ] `SHA256SUMS.txt` asset'lerle eşleşiyor.
- [ ] NSIS `.exe` ve (varsa) MSI yayımlandı.

Yayından sonra:

- [ ] Release sayfasında asset indirilebiliyor.
- [ ] Site sürüm numarasını ve indirme bağlantısını doğru gösteriyor.
- [ ] Updater endpoint'i (`releases/latest/download/latest.json`) erişilebilir.
- [ ] Kurulu uygulamada güncelleme denetimi yeni sürümü görüyor (sonraki
      sürümde gerçek güncelleme testi).
- [ ] Kod imzalama yoksa SmartScreen notu doğru.

Doğrulanmayan adımlar raporda açıkça belirtilir; "doğrulandı" yazılmaz.
