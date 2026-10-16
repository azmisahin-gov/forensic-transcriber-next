# ADR 0002 — Rust uygulama çekirdeği, ince arayüz

## Bağlam
Aynı iş kuralının hem arayüz hem arka uçta tekrarlanması tutarsızlığa ve
güvenlik açıklarına yol açar.

## Karar
Tüm iş kuralları `ftn-domain` ve `ftn-application` içindedir. React katmanı iş
kuralı içermez ve veritabanına doğrudan erişmez. Tauri komutları yalnızca
DTO dönüşümü ve doğrulama yapar.

## Sonuçlar
- Tek yetkili doğruluk kaynağı; test edilebilir çekirdek.
- Arayüz saf sunum katmanıdır.

## Alternatifler
Kuralları TypeScript'e koymak (güvenilmez; arayüz güven sınırı değildir).
