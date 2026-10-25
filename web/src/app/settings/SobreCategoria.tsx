import { useEffect, useState } from "react";
import { call } from "../../bridge";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { LinhasDeAtualizacao } from "../atualizacao/LinhasDeAtualizacao";

const LICENSE_URL = "https://www.gnu.org/licenses/agpl-3.0.html";
const SOURCE_URL = "https://github.com/lipanski/tako";

/** Abre o endereço no navegador do sistema: o hospedeiro manda janelas novas e links externos para fora do app. */
function openExternal(url: string) {
  window.open(url, "_blank", "noopener,noreferrer");
}

/** Configurações › Sobre: a versão (`app.info`), a licença e o código-fonte. Sem versão, a linha some. */
export function SobreCategoria() {
  const t = useT();
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    let cancelled = false;
    call("app.info")
      .then((info) => {
        if (!cancelled) setVersion(info.version);
      })
      .catch((error: unknown) => {
        console.error("app.info falhou", error);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <>
      <GrupoDeConfiguracao title={t("settings.about_web.app")}>
        {version !== null && (
          <LinhaDeConfiguracao title={t("settings.about.version")} control={<span>{version}</span>} />
        )}
        <LinhaDeConfiguracao
          title={t("settings.about.license")}
          description={t("settings.about.license_desc")}
          control={
            <>
              <span>{t("settings.about.license_value")}</span>
              <BotaoDesktop
                variant="outlined"
                onClick={() => {
                  openExternal(LICENSE_URL);
                }}
              >
                {t("settings.about_web.license_action")}
              </BotaoDesktop>
            </>
          }
        />
        <LinhaDeConfiguracao
          title={t("settings.about.source")}
          description={t("settings.about.source_desc")}
          control={
            <BotaoDesktop
              variant="outlined"
              onClick={() => {
                openExternal(SOURCE_URL);
              }}
            >
              {t("settings.about_web.source_action")}
            </BotaoDesktop>
          }
        />
      </GrupoDeConfiguracao>
      <LinhasDeAtualizacao />
    </>
  );
}
