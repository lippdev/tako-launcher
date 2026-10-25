import { useEffect, useState } from "react";
import { call } from "../../bridge";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { LinhasDeAtualizacao } from "../atualizacao/LinhasDeAtualizacao";
import { settingsStrings } from "../strings";

const s = settingsStrings.about;

/** Abre o endereço no navegador do sistema: o hospedeiro manda janelas novas e links externos para fora do app. */
function openExternal(url: string) {
  window.open(url, "_blank", "noopener,noreferrer");
}

/** Configurações › Sobre: a versão (`app.info`), a licença e o código-fonte. Sem versão, a linha some. */
export function SobreCategoria() {
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
      <GrupoDeConfiguracao title={s.app}>
        {version !== null && (
          <LinhaDeConfiguracao title={s.version} control={<span>{version}</span>} />
        )}
        <LinhaDeConfiguracao
          title={s.license.title}
          description={s.license.description}
          control={
            <>
              <span>{s.license.value}</span>
              <BotaoDesktop
                variant="outlined"
                onClick={() => {
                  openExternal(s.license.url);
                }}
              >
                {s.license.action}
              </BotaoDesktop>
            </>
          }
        />
        <LinhaDeConfiguracao
          title={s.source.title}
          description={s.source.description}
          control={
            <BotaoDesktop
              variant="outlined"
              onClick={() => {
                openExternal(s.source.url);
              }}
            >
              {s.source.action}
            </BotaoDesktop>
          }
        />
      </GrupoDeConfiguracao>
      <LinhasDeAtualizacao />
    </>
  );
}
