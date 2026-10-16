# Güvenlik Politikası

## Desteklenen sürümler

Proje erken aşamadadır. Güvenlik düzeltmeleri yalnızca en güncel sürüme uygulanır.

## Açık bildirimi

Bir güvenlik açığı bulduysanız, lütfen herkese açık bir issue açmadan önce
sorumlu bildirim yapın. Bildiriminizde şunları belirtin:

- Etkilenen bileşen ve sürüm.
- Sorunun türü (ör. yol geçişi, komut enjeksiyonu, veri bütünlüğü ihlali).
- Yeniden üretme adımları.
- Etki değerlendirmesi.

Acil bir iletişim kanalı henüz tanımlı değildir; bu belge, bir kanal
belirlenene kadar geçerlidir ve güncellenecektir.

## Tasarım güvenlik ilkeleri

- Arayüz güven sınırı değildir; tüm girdiler Rust tarafında doğrulanır.
- Alt süreçler kabuk olmadan, argüman dizisiyle başlatılır.
- En az yetki: yalnızca `core:default` Tauri izni verilir; dosya sistemi, kabuk,
  diyalog ve HTTP eklentileri kullanılmaz.
- Kısıtlı CSP.
- Telemetri ve gizli ağ çağrısı yoktur.

Ayrıntı için `docs/security/threat-model.md` ve `docs/security/permissions.md`.
