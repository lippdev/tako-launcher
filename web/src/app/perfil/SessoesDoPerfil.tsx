import { useCallback, useMemo, useState } from "react";
import { call } from "../../bridge";
import type { SessionPeriod } from "../../bridge/generated/SessionPeriod";
import type { Aba } from "../../components/Abas/Abas";
import { PerfilSessoes } from "../../components/PerfilSessoes/PerfilSessoes";
import { useT } from "../../i18n";
import { PERIODS, sessionGroups, sessionsSummary } from "./perfilModel";
import { useBridgeData } from "./useBridgeData";
import { useCapas } from "./useCapas";
import styles from "./TelaPerfil.module.css";

interface SessoesDoPerfilProps {
  onOpenGame: (id: string) => void;
}

/** Perfil › Sessões (README do PerfilSessoes): o período, o resumo e as sessões agrupadas por dia. */
export function SessoesDoPerfil({ onOpenGame }: SessoesDoPerfilProps) {
  const t = useT();
  const cover = useCapas(40, 56);
  const [period, setPeriod] = useState<SessionPeriod>("week");
  const load = useCallback(() => call("profile.sessions", { period }), [period]);
  const { data, status, reload } = useBridgeData(load);

  const tabs: readonly Aba<SessionPeriod>[] = PERIODS.map((id) => ({ id, label: t(`profile.period_${id}`) }));
  const groups = useMemo(
    () =>
      data
        ? sessionGroups(data.days, { today: data.today, t, cover }).map((group) => ({
            ...group,
            sessions: group.sessions.map((session, i) => ({
              ...session,
              onClick: () => {
                const game = data.days.find((day) => day.day === group.id)?.sessions[i]?.game;
                if (game) onOpenGame(game.id);
              },
            })),
          }))
        : [],
    [data, t, cover, onOpenGame],
  );

  if (status === "error") {
    return (
      <div className={styles["aviso"]} role="alert">
        {t("profile.web.load_error")}
        <button type="button" className={styles["retry"]} onClick={reload}>
          {t("profile.web.retry")}
        </button>
      </div>
    );
  }

  return (
    <PerfilSessoes
      period={{ label: t("profile.web.period"), tabs, value: period, onChange: setPeriod }}
      toolbarLabel={t("profile.web.sessions_toolbar")}
      {...(data ? { summary: sessionsSummary(data.count, data.totalSeconds, t) } : {})}
      groups={groups}
      {...(data && data.count === 0 ? { emptyMessage: t("profile.empty") } : {})}
    />
  );
}
