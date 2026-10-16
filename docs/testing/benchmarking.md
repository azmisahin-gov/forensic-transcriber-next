# Performans ve Doğruluk Ölçümü

## İlkeler

- Ölçmeden yüzde veya süre garantisi verilmez.
- Doğruluk iddiaları yalnızca doğrulanmış bir referans kümesiyle desteklenir.

## Performans

Hedefler (henüz ölçülmedi):

- Ses dönüştürme ve transkripsiyon arayüzü bloke etmez.
- Veritabanı işlemleri dar kapsamlıdır.
- Büyük dosyalar belleğe tamamen yüklenmez (hash ve kopyalama 1 MiB parçalarla).
- İlerleme gerçek süreç durumundan üretilir.

Ölçüm için `tests/benchmarks/` dizini ayrılmıştır; içerik henüz yoktur. Ölçüm
yapıldığında yöntem ve sonuçlar buraya yazılmalıdır.

## Doğruluk (WER/CER)

Gerçek Türkçe konuşma doğruluğunu değerlendirmek için **doğrulanmış referans
metinli** bir test veri kümesi gerekir.

> **Durum: böyle bir veri kümesi mevcut değildir.** Bu nedenle depoda hiçbir
> doğruluk yüzdesi iddia edilmez. Arayüzde de gösterilmez.

Planlanan yöntem:

1. Anonimleştirilmiş ve izinli, referans metni doğrulanmış kayıtlar toplanır.
2. Her kayıt için motor çıktısı üretilir.
3. WER/CER, normalize edilmiş metin üzerinden hesaplanır (noktalama ve büyük/küçük
   harf normalizasyonu ayrı raporlanır).
4. Sonuçlar model/sürüm bazında saklanır ve `docs/testing/` altında yayımlanır.

Sentetik (TTS) ses ile yapılan ölçümler gerçek dünya doğruluğu olarak
sunulamaz.
