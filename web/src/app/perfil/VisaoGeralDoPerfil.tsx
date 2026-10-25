import { useEffect, useMemo, useState } from "react";
import type { ProfileSummary } from "../../bridge/generated/ProfileSummary";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { FileiraDeCapas, type CapaDaFileira } from "../../components/FileiraDeCapas/FileiraDeCapas";
import { BarrasDaSemana, Bloco, GrupoDeBlocos } from "../../components/GrupoDeBlocos/GrupoDeBlocos";
import { PerfilSessoes } from "../../components/PerfilSessoes/PerfilSessoes";
import { useT } from "../../i18n";
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
 * Os blocos montam em sequência para não sobrecarregar o primeiro quadro com múltiplos vidros e elementos.
 */
export function VisaoGeralDoPerfil({ summary, onOpenGame, onSeeAll }: VisaoGeralDoPerfilProps) {
  const t = useT();
  const cover = useCapas(40, 56);
  const shelfCover = useCapas(233, 311);
  const [stage, setStage] = useState(0);

  useEffect(() => {
    let frame = requestAnimationFrame(() => {
      setStage(1);
      frame = requestAnimationFrame(() => {
        setStage(2);
      });
    });
    return () => {
      cancelAnimationFrame(frame);
    };
  }, []);

  const bars = useMemo(() => weekBars(summary.week, summary.today, t), [summary.week, summary.today, t]);
  const days = useMemo(() => bars.map((bar) => ({ label: bar.label, value: bar.value })), [bars]);
  const totalPlay = useMemo(() => playTimeText(summary.totalPlaySeconds, t), [summary.totalPlaySeconds, t]);
  const totalWeek = useMemo(() => weekTotal(summary.week), [summary.week]);
  const weekLabel = useMemo(
    () => `${t("profile.this_week")} · ${durationText(totalWeek, t)}`,
    [t, totalWeek],
  );
  const weekAria = useMemo(
    () => weekAriaLabel(summary.week, summary.today, t),
    [summary.week, summary.today, t],
  );

  const top: CapaDaFileira[] = useMemo(
    () =>
      summary.topGames.map((game) => ({
        id: game.id,
        title: game.title,
        label: playTimeText(game.playTimeSeconds, t),
        accentColor: game.accent,
        hasArt: game.hasCover,
        src: shelfCover(game),
      })),
    [summary.topGames, shelfCover, t],
  );

  const recent = useMemo(
    () =>
      summary.recentSessions.map((session, index) => ({
        ...historySession(session, index, { today: summary.today, t, cover, overview: true }),
        onClick: () => {
          onOpenGame(session.game.id);
        },
      })),
    [summary.recentSessions, summary.today, t, cover, onOpenGame],
  );

  const recentGroups = useMemo(
    () => (recent.length > 0 ? [{ id: "recent", title: "", total: "", sessions: recent }] : []),
    [recent],
  );

  return (
    <>
      <GrupoDeBlocos>
        <Bloco label={t("profile.tile_time")} icon="clock">
          {totalPlay}
        </Bloco>
        <Bloco label={weekLabel}>
          <div className={styles["semana"]}>
            <BarrasDaSemana days={days} aria-label={weekAria} />
          </div>
        </Bloco>
        <Bloco label={t("profile.tile_games")} icon="grid">
          {summary.gameCount}
        </Bloco>
      </GrupoDeBlocos>

      {stage >= 1 && top.length > 0 && (
        <div className={styles["secao"]}>
          <FileiraDeCapas
            title={t("profile.tile_top")}
            covers={top}
            previousLabel={t("common.previous")}
            nextLabel={t("common.next")}
            onOpen={onOpenGame}
          />
        </div>
      )}

      {stage >= 2 && (
        <section className={styles["secao"]} aria-label={t("profile.web.recent_list")}>
          <div className={styles["secaoCabeca"]}>
            <h2 className={styles["secaoTitulo"]}>{t("profile.recent")}</h2>
            {recent.length > 0 && (
              <BotaoDesktop variant="outlined" onClick={onSeeAll}>
                {t("desktop.sessions.see_all")}
              </BotaoDesktop>
            )}
          </div>
          <PerfilSessoes groups={recentGroups} emptyMessage={t("profile.empty")} />
        </section>
      )}
    </>
  );
}
