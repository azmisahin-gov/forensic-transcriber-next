import type { StartupStatusDto } from "../../services/ipc";

const ENGINE_ENV = "FTN_ENGINE_PATH";

/**
 * Shown when no transcription engine is configured. It is honest about what the
 * app can and cannot do without an engine, and gives the exact steps to enable
 * transcription. Everything below is real: the app stores cases, imports and
 * hashes evidence, and exports transcripts without an engine.
 */
export function PreparationPanel({ startup }: { startup: StartupStatusDto | null }) {
  if (!startup || startup.engineAvailable) {
    return null;
  }
  return (
    <section className="panel preparation" aria-labelledby="prep-heading">
      <h2 id="prep-heading">Transkripsiyon motoru kurulu değil</h2>
      <p>
        Uygulama çalışıyor. Vaka oluşturma, ses içe aktarma, <strong>SHA-256
        bütünlük doğrulama</strong>, kayıt ve dışa aktarma motor olmadan da
        kullanılabilir. Yalnızca <strong>otomatik transkripsiyon</strong> için
        yerel bir whisper.cpp kurulumu gerekir.
      </p>
      <ol className="prep-steps">
        <li>
          whisper.cpp'yi indirip derleyin veya hazır bir <span className="mono">whisper-cli</span>{" "}
          yürütülebilir dosyası edinin.
        </li>
        <li>
          Bir GGML model dosyası (<span className="mono">.bin</span>) indirin ve
          yolunu not edin.
        </li>
        <li>
          <span className="mono">{ENGINE_ENV}</span> ortam değişkenini{" "}
          <span className="mono">whisper-cli</span> yoluna ayarlayıp uygulamayı
          yeniden başlatın.
        </li>
        <li>
          Transkripsiyon ekranındaki <em>Model yolu</em> alanına model dosyasını
          girin.
        </li>
      </ol>
      <p className="prep-note">
        Motor bulunamadığında uygulama örnek/uydurma transkript göstermez; durumu
        açıkça bildirir. Bu sürümde gerçek whisper.cpp koşusu bu makinede
        doğrulanmamıştır.
      </p>
      {startup.engineNote ? (
        <p className="muted mono">Tanılama: {startup.engineNote}</p>
      ) : null}
    </section>
  );
}
