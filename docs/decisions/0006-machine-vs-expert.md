# ADR 0006 — Otomatik ve uzman çıktısının ayrılması

## Bağlam
Makine çıktısı ile uzmanın onayladığı metin karıştırılırsa inceleme izlenemez
hale gelir ve yanlış kesinlik izlenimi doğar.

## Karar
Her segment `original_text` (motor çıktısı, değişmez) ve `text` (güncel) alanlarını
taşır. Durumlar `AUTOMATIC / REVIEWED / EDITED / VERIFIED`'dir. Değişiklikler
`transcript_revisions` ile değişmez anlık görüntüler olarak saklanır; mevcut
onaylanmış sürüm sessizce ezilmez. Dışa aktarımlar otomatik/insan sayılarını
bildirir.

## Sonuçlar
- İnceleme geçmişi denetlenebilir.
- Düzenleme asla otomatik metni yok etmez.
- Onaylanmış (VERIFIED) durum sessizce düşürülmez.

## Alternatifler
Tek `text` alanı (denetlenebilirlik kaybı).
