# ADR 0001 — Masaüstü kabuğu olarak Tauri 2

## Bağlam
Eski proje Electron kullanıyordu. Yeni uygulama yerel, hafif ve güvenli bir
masaüstü kabuğu gerektiriyor; ağır Node çalışma zamanını paketlememelidir.

## Karar
Tauri 2 kullanılır. İş mantığı Rust'ta; arayüz web teknolojileriyle.

## Sonuçlar
- Küçük paket, sistem WebView2 kullanımı.
- Rust güvenlik modeli ve yetenek (capability) sistemi.
- Olgun ancak Electron'a göre daha genç ekosistem.

## Alternatifler
Electron (daha büyük paket, daha fazla saldırı yüzeyi), native GUI (daha yavaş
geliştirme).
