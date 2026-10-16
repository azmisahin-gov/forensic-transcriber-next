# ADR 0003 — Veri deposu olarak SQLite

## Bağlam
Uygulama tamamen yerel çalışır; sunucu veya bulut yoktur. İlişkisel bir vakalar,
kanıtlar, segmentler ve denetim kayıtları modeli gerekir.

## Karar
SQLite (rusqlite, `bundled`) kullanılır. Şema Rust tarafından yönetilir;
parametreli sorgular ve sürümlenmiş migrasyonlar zorunludur. Foreign key etkindir.

## Sonuçlar
- Tek dosya, yedeklemesi kolay, çevrimdışı.
- Transaction ve foreign key ile veri bütünlüğü.
- Eşzamanlı çoklu yazıcı sınırlı (tek kullanıcı için yeterli).

## Alternatifler
Düz dosya/JSON (sorgulama ve bütünlük zayıf), gömülü bir NoSQL (ilişkisel model
kaybı).
