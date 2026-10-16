# ADR 0005 — Orijinal kanıtın korunması

## Bağlam
Bir kanıt dosyasının gizli veya kazara değiştirilmesi geri alınamaz sonuçlar
doğurur. Kullanıcının seçtiği dosyaya yazmak kabul edilemez.

## Karar
İçe aktarma; hash → geçici dosyaya kopya → kopyayı yeniden hash'le →
doğrula → atomik rename akışını izler. Kaynak dosyaya yalnızca okunur erişilir.
Orijinal ve türetilmiş dosya ayrı dizinlerde tutulur; türetilmiş varlığın
üretim parametreleri ve özeti kaydedilir.

## Sonuçlar
- Bütünlük ihlali sessizce geçilemez.
- Kesilen kopyalama yarım dosya bırakmaz.
- SHA-256 kaydı köken/izlenebilirlik özelliğidir; adli zincir garantisi değildir.

## Alternatifler
Yerinde çalışmak (veri kaybı riski), yalnızca yol referansı (bütünlük izlenemez).
