import { useMemo } from "react";
import type { ProfileSummary } from "../../bridge/generated/ProfileSummary";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { FileiraDeCapas, type CapaDaFileira } from "../../components/FileiraDeCapas/FileiraDeCapas";
import { BarrasDaSemana, Bloco, GrupoDeBlocos } from "../../components/GrupoDeBlocos/GrupoDeBlocos";
import { PerfilSessoes } from "../../components/PerfilSessoes/PerfilSessoes";
import { useT } from "../../i18n";
import { profileStrings as s } from "../strings";
import { durationText, historySession, playTimeText, weekAriaLabel, weekBars, weekTotal } from "./perfilModel";
import { useCapas } from "./useCapas";
import styles from "./TelaPerfil.module.css";

interface VisaoGeralDoPerfilProps {
  summary: ProfileSummary;
  onOpenGame: (id: string) => void;
  /** "Ver todas": abre a guia Sessões. */
  onSeeAll: () => void;
}

/**
 * Perfil › Visão geral (README da TelaPerfil): os blocos com o tempo jogado, a semana e os jogos da biblioteca, o
 * que mais se jogou e as últimas sessões. O que o núcleo não respondeu não vira bloco.
 */
export function VisaoGeralDoPerfil({ summary, onOpenGame, onSeeAll }: VisaoGeralDoPerfilProps) {
  const t = useT();
  const cover = useCapas(40, 56);
  const shelfCover = useCapas(233, 311);

  const bars = useMemo(() => weekBars(summary.week, summary.today, t), [summary.week, summary.today, t]);
  const top: CapaDaFileira[] = summary.topGames.map((game) => ({
    id: game.id,
    title: game.title,
    label: playTimeText(game.playTimeSeconds, t),
    accentColor: game.accent,
    hasArt: game.hasCover,
    src: shelfCover(game),
  }));
  const recent = summary.recentSessions.map((session, index) => ({
    ...historySession(session, index, { today: summary.today, t, cover, overview: true }),
    onClick: () => {
      onOpenGame(session.game.id);
    },
  }));

  return (
    <>
      <GrupoDeBlocos>
        <Bloco label={t("profile.tile_time")} icon="clock">
          {playTimeText(summary.totalPlaySeconds, t)}
        </Bloco>
        <Bloco label={`${t("profile.this_week")} · ${durationText(weekTotal(summary.week), t)}`}>
          <div className={styles["semana"]}>
            <BarrasDaSemana
              days={bars.map((bar) => ({ label: bar.label, value: bar.value }))}
              aria-label={weekAriaLabel(summary.week, summary.today, t)}
            />
          </div>
        </Bloco>
        <Bloco label={t("profile.tile_games")} icon="grid">
          {summary.gameCount}
        </Bloco>
      </GrupoDeBlocos>

      {top.length > 0 && (
        <div className={styles["secao"]}>
          <FileiraDeCapas
            title={t("profile.tile_top")}
            covers={top}
            previousLabel={s.previous}
            nextLabel={s.next}
            onOpen={onOpenGame}
          />
        </div>
      )}

      <section className={styles["secao"]} aria-label={s.recentList}>
        <div className={styles["secaoCabeca"]}>
          <h2 className={styles["secaoTitulo"]}>{t("profile.recent")}</h2>
          {recent.length > 0 && (
            <BotaoDesktop variant="outlined" onClick={onSeeAll}>
              {t("desktop.sessions.see_all")}
            </BotaoDesktop>
          )}
        </div>
        <PerfilSessoes
          groups={recent.length > 0 ? [{ id: "recent", title: "", total: "", sessions: recent }] : []}
          emptyMessage={t("profile.empty")}
        />
      </section>
    </>
  );
}
