# Veri Modeli

Depo şeması sürüm 1'dir (`ftn_storage::SCHEMA_VERSION`). Migrasyonlar eklemeli
(append-only) ve transaction içindedir; `user_version` pragma'sı ile izlenir.

## Varlıklar

### cases
| Sütun | Tip | Not |
| --- | --- | --- |
| case_id | TEXT PK | UUID |
| title | TEXT | boş olamaz |
| notes | TEXT | |
| created_at / updated_at | INTEGER | epoch ms |

### evidence
| Sütun | Tip | Not |
| --- | --- | --- |
| evidence_id | TEXT PK | |
| case_id | TEXT FK→cases | ON DELETE CASCADE |
| original_name / stored_name | TEXT | orijinal ve yönetilen ad |
| original_path / stored_path | TEXT | orijinal yola dokunulmaz |
| size_bytes | INTEGER | |
| format, codec, duration_seconds, sample_rate, channels, bit_depth, audio_stream_count | | FFprobe; bilinemiyorsa NULL |
| source_sha256 | TEXT | kaynak dosyanın özeti (giriş anında) |
| stored_sha256 | TEXT | kopyanın doğrulanmış özeti |
| integrity | TEXT | VERIFIED / MISMATCH / MISSING / UNCHECKED |
| imported_at | INTEGER | epoch ms |

### derived_assets
Kaynak→türetilmiş ilişkisi, üretim parametreleri (`generator`) ve hash.

### transcripts
`(case_id, evidence_id)` UNIQUE. Motor ve model bilgisini taşır. Aynı kanıt için
ikinci transkript oluşturulmaz; bu, otomatik çıktının sessizce üzerine yazılmasını
engeller.

### segments
PK `segment_id`; `transcript_id` FK. Zaman `start_ms`/`end_ms` **tam sayı
milisaniyedir** (kayan nokta belirsizliğini önlemek için). `CHECK (end_ms >
start_ms)` ve `CHECK (start_ms >= 0)` vardır. Çakışmasızlık kuralı **yoktur**;
gerçek konuşmalarda üst üste konuşma olabilir.

- `text`: güncel (düzenlenmiş olabilir) metin.
- `original_text`: motor çıktısı; bir kez yazılır ve asla üzerine yazılmaz.
- `status`: AUTOMATIC / REVIEWED / EDITED / VERIFIED.
- `flags_json`: operatör çalışma işaretleri (ör. UNCLEAR).

### transcript_revisions
Değişmez anlık görüntüler. Transkript başına **tam olarak bir** satır `is_current=1`.
Yeni ASR koşusu veya kayıt mevcut revizyonu ezmez, yeni revizyon ekler.

### transcription_jobs
Her deneme için bir satır. `status`: QUEUED / RUNNING / SUCCEEDED / FAILED /
CANCELLED. Başarısız/iptal/kuyruk/tamamlanmış durumları ayrıdır. Motor, sürüm,
model ve ayarlar kaydedilir.

### audit_events
Vaka/kanıt/transkript/segment bağlantılı; `before_json`/`after_json` ile
değişiklik. `actor_label` yalnızca kullanıcı tarafından sağlanan bir etikettir,
doğrulanmış bir kimlik değildir.

### export_records
Üretilen dosya, biçim, şema sürümü ve hash.

### application_settings
Yerel anahtar/değer ayarları. Makineden ayrılmaz.

## İlkeler

- Orijinal dosyanın içeriği asla yerinde değiştirilmez.
- Orijinal ile çalışma kopyası ayrı kavramlardır.
- Otomatik ve uzman metni ayrı tutulur.
- Zamanlar tam sayı milisaniye olarak saklanır.
- Foreign key etkindir; çok adımlı yazımlar transaction içindedir.
- Yarım yazılmış kayıt tamamlanmış gibi gösterilmez.

## `SchemaMigration`

Şema sürümü SQLite `user_version` pragma'sında tutulur. Her migrasyon
`(from_version, to_version)` bilgisiyle sıralı uygulanır; ileri sürüm algılanırsa
`SchemaTooNew` hatası verilir ve veri bozulmaz.
