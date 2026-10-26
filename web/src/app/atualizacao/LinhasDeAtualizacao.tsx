import { useEffect, useState } from "react";
import { call } from "../../bridge";
import type { UpdatePrefs } from "../../bridge/generated/UpdatePrefs";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { LinhaChave } from "../settings/Linhas";
import { menuLine } from "./atualizacaoModel";
import { checkForUpdates, useUpdates } from "./atualizacaoStore";

const KEYS = [
  { key: "autoUpdate", text: "settings.general.autoupdate" },
  { key: "prereleases", text: "settings.general.prerelease" },
  { key: "nightly", text: "settings.general.nightly" },
] as const;

/**
 * Configurações › Sobre › Atualizações: procurar agora e as três preferências de `updates.json` (as mesmas do
 * Configurações › Geral do Slint). Gravar é pela ponte (`updates.preferences.set`); o núcleo devolve o que ficou.
 */
export function LinhasDeAtualizacao() {
  const t = useT();
  const { status, transient } = useUpdates();
  const [prefs, setPrefs] = useState<UpdatePrefs | null>(null);

  useEffect(() => {
    let cancelled = false;
    call("updates.preferences")
      .then((saved) => {
        if (!cancelled) setPrefs(saved);
      })
      .catch((error: unknown) => {
        console.error("atualização: preferências", error);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const save = (patch: Partial<UpdatePrefs>) => {
    if (prefs === null) return;
    call("updates.preferences.set", { ...prefs, ...patch })
      .then(setPrefs)
      .catch((error: unknown) => {
        console.error("atualização: gravar preferências", error);
      });
  };

  const line = menuLine(status, transient, t("settings.general.updates_desc"), t);
  return (
    <GrupoDeConfiguracao title={t("settings.general.updates_header")}>
      <LinhaDeConfiguracao
        title={t("settings.general.updates")}
        description={line.label}
        control={
          <BotaoDesktop
            variant="outlined"
            disabled={line.disabled}
            onClick={() => {
              void checkForUpdates();
            }}
          >
            {t("settings.general.updates_value")}
          </BotaoDesktop>
        }
      />
      {prefs !== null &&
        KEYS.map(({ key, text }) => (
          <LinhaChave
            key={key}
            title={t(text)}
            description={t(`${text}_desc`)}
            checked={prefs[key]}
            onChange={(checked) => {
              save({ [key]: checked });
            }}
          />
        ))}
    </GrupoDeConfiguracao>
  );
}
