# Yöntem: Orijinal Kayıt, Çalışma Kopyası, Hash ve İnceleme

## Orijinal kayıt korunur

İçe aktarma sırasında kullanıcının dosyası **hiçbir zaman değiştirilmez**. Akış:

1. Kullanıcının seçtiği dosyanın yolu ve temel metadata'sı kaydedilir.
2. Orijinal dosyanın SHA-256 değeri hesaplanır.
3. Yönetilen kanıt alanına bir kopya alınır (geçici dosyaya, sonra rename).
4. Kopyanın SHA-256 değeri yeniden hesaplanır.
5. Başlangıç ve kopya özetleri karşılaştırılır.
6. Uyuşmazlıkta dosya **doğrulanmış sayılmaz**; hata görünür biçimde raporlanır.
7. Türetilmiş çalışma sesi orijinalden ayrı üretilir.
8. Türetilmiş varlığın kaynağı, üretim parametreleri ve özeti kaydedilir.

Bu nedenle orijinal, yalnızca okunur kanıt olarak kalır.

## Bütünlük doğrulama

- `verify_evidence_integrity` saklanan kopyayı yeniden hash'ler ve giriş anındaki
  özetle karşılaştırır.
- Sonuç: `VERIFIED`, `MISMATCH` veya `MISSING`.
- Sonuç hem veritabanına hem denetim kaydına yazılır.

**Önemli sınır:** SHA-256 kaydı bir yazılım köken/izlenebilirlik özelliğidir.
Tam bir adli zincir veya hukuki geçerlilik garantisi **değildir**.

## Otomatik ve uzman çıktısı ayrımı

- Segmentin `original_text` alanı motor çıktısıdır ve bir kez yazılır.
- `text` alanı güncel (düzenlenmiş olabilir) metindir.
- Durumlar: `AUTOMATIC`, `REVIEWED`, `EDITED`, `VERIFIED`.
- Düzenleme otomatik metni silmez; arayüz farkı gösterir.
- `transcript_revisions` her önemli kayıtta değişmez bir anlık görüntü ekler;
  onaylanmış (VERIFIED) iş sessizce ezilmez.

## İnceleme yaklaşımı

- Anlaşılamayan bölümler açıkça işaretlenir (ör. `[ANLAŞILAMADI]`); uygulama
  duyulmayan sözü kendiliğinden tamamlamaz.
- Tahmini veya değiştirilmiş metin, orijinal sesle karıştırılmaz.
- Konuşmacı etiketleri çalışma konvansiyonudur; biyometrik kimlik tespiti
  değildir.

## Denetlenebilirlik

Her önemli değişiklik `audit_events` içine yazılır: olay türü, önce/sonra
değerleri, zaman, uygulama sürümü ve varsa operatör etiketi. `actor_label`
doğrulanmış bir kimlik değildir; uygulama kimlik doğrulaması sunmaz.
