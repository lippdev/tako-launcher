import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import type { GameDetails } from "../../bridge/generated/GameDetails";
import type { LibraryGame } from "../../bridge/generated/LibraryGame";
import mark from "../../assets/brand/tako-mark.png";
import { AcoesDoJogo, type EstadoDoPrimario } from "../../components/AcoesDoJogo/AcoesDoJogo";
import { Aviso, type AvisoTone } from "../../components/Aviso/Aviso";
import { BarraDeDados, type DadoDaBarra } from "../../components/CartaoSessoes/CartaoSessoes";
import { Destaque } from "../../components/Destaque/Destaque";
import { FileiraDeCapas, type CapaDaFileira } from "../../components/FileiraDeCapas/FileiraDeCapas";
import { FundoDoJogo } from "../../components/FundoDoJogo/FundoDoJogo";
import { PilulaDaLoja } from "../../components/PilulaDaLoja/PilulaDaLoja";
import { TituloDoJogo } from "../../components/TituloDoJogo/TituloDoJogo";
import { useT } from "../../i18n";
import { cx } from "../../lib/cx";
import { EtiquetaDeSituacao } from "../jogo/EtiquetaDeSituacao";
import {
  formatLastPlayed,
  formatPlayTime,
  metacriticTone,
  metacriticVerdict,
  paragraphs,
  primaryState,
  validScore,
} from "../jogo/jogoModel";
import { useDevImages } from "../jogo/useDevImages";
import { useGameSession } from "../sessao/useGameSession";
import type { GamePhase } from "../sessao/sessionModel";
import { useDetalhesDoDestaque } from "./useDetalhesDoDestaque";
import { useVisaoGeral } from "./useVisaoGeral";
import { CONTINUE_ID, RECENT_ID, gamesById, visibleShelves, type ShelfState } from "./visaoModel";
import styles from "./VisaoGeral.module.css";

interface VisaoGeralProps {
  /** Abrir a página do jogo (o mesmo caminho da lista à esquerda). */
  onOpenGame: (id: string) => void;
}

interface Notice {
  open: boolean;
  tone: AvisoTone;
  text: string;
  key: number;
}

/** Passar o mouse por cima de várias capas seguidas não troca o destaque a cada uma: ele espera o mouse parar. */
const HOVER_INTENT_MS = 120;

/** Espera as transições das partes do destaque saírem; sem transição (reduzir animações), já resolve. */
function settleParts(root: HTMLElement | null): Promise<unknown> {
  if (!root) return Promise.resolve();
  const animations = [...root.querySelectorAll("[data-parte]")].flatMap((part) => part.getAnimations());
  return Promise.allSettled(animations.map((animation) => animation.finished));
}

function shelfTitle(shelf: ShelfState, t: (key: string) => string): string {
  switch (shelf.spec.id) {
    case CONTINUE_ID:
      return t("desktop.library_overview.continue_playing");
    case RECENT_ID:
      return t("desktop.library_overview.recent");
    default:
      return shelf.spec.name ?? "";
  }
}

/** Dados embaixo à esquerda do destaque: o que a lista já sabe, mais a nota quando os detalhes chegaram. */
function heroData(game: LibraryGame, details: GameDetails | null, t: (key: string) => string): DadoDaBarra[] {
  const last = formatLastPlayed(game.lastPlayedAt);
  const time = formatPlayTime(game.playTimeSeconds);
  const score = validScore(details?.metacriticScore ?? null);
  return [
    ...(last ? [{ id: "last", icone: "calendar", label: t("desktop.strip.last_session"), valor: last } as const] : []),
    ...(time ? [{ id: "time", icone: "clock", label: t("details.play_time"), valor: time } as const] : []),
    ...(score !== null
      ? [
          {
            id: "metacritic",
            icone: "badge",
            label: t("details.metacritic"),
            valor: t(`game_page.verdict_${metacriticVerdict(score)}`),
            nota: { valor: score, faixa: metacriticTone(score) },
          } as const,
        ]
      : []),
  ];
}

/**
 * A Visão geral da biblioteca (README da TelaBiblioteca): o destaque do jogo em foco e, embaixo, as fileiras de capas
 * (continuar jogando, recentes e as coleções). O destaque começa no jogo jogado por último e troca para a capa que
 * recebe o mouse ou o foco, com o movimento do TrocaDeJogo; o que o núcleo não respondeu não vira bloco.
 */
export function VisaoGeral({ onOpenGame }: VisaoGeralProps) {
  const overview = useVisaoGeral();
  const t = useT();
  const session = useGameSession();
  const devImage = useDevImages();
  const heroRef = useRef<HTMLDivElement>(null);
  const [focusId, setFocusId] = useState<string | null>(null);
  const [shownId, setShownId] = useState<string | null>(null);
  const [entered, setEntered] = useState(false);
  const [notice, setNotice] = useState<Notice>({ open: false, tone: "info", text: "", key: 0 });

  const games = useMemo(() => gamesById(overview.shelves, overview.first), [overview.shelves, overview.first]);
  const shelves = useMemo(() => visibleShelves(overview.shelves), [overview.shelves]);

  // O foco é o jogo da capa que recebeu o mouse; sem ele (ou se o jogo saiu da biblioteca), o jogado por último.
  const targetId = focusId !== null && games.has(focusId) ? focusId : (overview.first?.id ?? null);
  const entries = useDetalhesDoDestaque(targetId);
  const targetReady = targetId !== null && entries[targetId] !== undefined;

  // O destaque só troca quando o jogo novo já tem detalhes (ou a falha deles): nada de pular sem a descrição.
  if (targetReady && (shownId === null || !games.has(shownId))) setShownId(targetId);
  const swapping = targetReady && shownId !== null && shownId !== targetId;

  useEffect(() => {
    if (!swapping) return;
    let cancelled = false;
    void settleParts(heroRef.current).then(() => {
      if (!cancelled) setShownId(targetId);
    });
    return () => {
      cancelled = true;
    };
  }, [swapping, targetId]);

  // A primeira vez as partes também entram em escada: começam fora e, dois quadros depois, voltam.
  const ready = shownId !== null;
  useEffect(() => {
    if (!ready) return;
    let frame = requestAnimationFrame(() => {
      frame = requestAnimationFrame(() => {
        setEntered(true);
      });
    });
    return () => {
      cancelAnimationFrame(frame);
    };
  }, [ready]);

  const hoverTimer = useRef<number | undefined>(undefined);
  const highlight = useCallback((id: string) => {
    window.clearTimeout(hoverTimer.current);
    hoverTimer.current = window.setTimeout(() => {
      setFocusId(id);
    }, HOVER_INTENT_MS);
  }, []);
  useEffect(
    () => () => {
      window.clearTimeout(hoverTimer.current);
    },
    [],
  );

  const showNotice = (tone: AvisoTone, text: string) => {
    setNotice({ open: true, tone, text, key: Date.now() });
  };
  const play = async (game: LibraryGame) => {
    if (session.phaseOf(game.id) !== "none") return;
    const feedback = await session.play(game);
    showNotice(feedback.tone, feedback.text);
  };

  const game = shownId !== null ? games.get(shownId) : undefined;
  const phase = game ? session.phaseOf(game.id) : "none";
  const details = shownId !== null ? (entries[shownId]?.details ?? null) : null;
  const covers = (shelf: ShelfState): CapaDaFileira[] =>
    shelf.games.map((item) => {
      const time = formatPlayTime(item.playTimeSeconds);
      const src = devImage?.("cover", item.id, item.title);
      return {
        id: item.id,
        title: item.title,
        label: time ? t("desktop.library_overview.played_for", { time }) : null,
        accentColor: item.accent,
        hasArt: item.hasCover,
        src,
      };
    });

  const empty = overview.status === "ready" && overview.total === 0;
  const headCount =
    overview.status === "loading" ? t("desktop.empty.loading") : overview.status === "ready" ? t("common.games", { n: overview.total }) : null;

  const openHero = (event: MouseEvent<HTMLDivElement>) => {
    // Clicar numa ação (Jogar) ou no menu de uma camada não abre a página; o resto do destaque abre.
    const target = event.target as Element;
    if (!event.currentTarget.contains(target) || target.closest("button, a, [role='menu']")) return;
    if (game) onOpenGame(game.id);
  };

  return (
    <div className={styles["pagina"]}>
      <div className={styles["cabeca"]}>
        <h1 className="page-title">{t("desktop.library_overview.welcome")}</h1>
        {headCount && <span className={styles["total"]}>{headCount}</span>}
      </div>

      {overview.status === "error" ? (
        <div className={styles["aviso"]} role="alert">
          {t("library.load_error")}
          <button type="button" className={styles["retry"]} onClick={overview.retry}>
            {t("library.retry")}
          </button>
        </div>
      ) : empty ? (
        <div className={styles["vazia"]}>
          <img className={styles["polvo"]} src={mark} alt="" />
          <p>{t("library.empty_library")}</p>
        </div>
      ) : !game ? (
        <Esqueleto label={t("desktop.empty.loading")} />
      ) : (
        <>
          {/* O clique no destaque é um atalho do mouse; pelo teclado o jogo se abre pela capa ou pela lista. */}
          <div ref={heroRef} className={styles["destaque"]} onClick={openHero}>
            <Destaque
              variante="visao"
              aria-label={t("desktop.library_overview.hero_label", { title: game.title })}
              trocando={swapping || !entered}
              fundo={
                <FundoDoJogo
                  id={game.id}
                  temArte={game.hasHero}
                  corDestaque={game.accent}
                  src={devImage?.("hero", game.id, game.title)}
                />
              }
              chips={<PilulaDaLoja label={t(`sources.store.${game.store}`)} />}
              situacao={
                phase !== "none" ? (
                  <EtiquetaDeSituacao
                    texto={session.elapsed ? `${t("game_page.running")} · ${session.elapsed}` : t("game_page.running")}
                    ligado
                  />
                ) : game.installed && game.lastPlayedAt !== null ? (
                  <EtiquetaDeSituacao texto={t("desktop.library_overview.continue_playing")} ligado />
                ) : (
                  <EtiquetaDeSituacao
                    texto={game.installed ? t("home.installed") : t("game.not_installed")}
                    ligado={game.installed}
                  />
                )
              }
              title={
                <TituloDoJogo
                  id={game.id}
                  title={game.title}
                  hasLogo={game.hasLogo}
                  size="visao"
                  src={devImage?.("logo", game.id, game.title)}
                />
              }
              descricao={paragraphs(details?.description ?? null)[0]}
              dados={<BarraDeDados dados={heroData(game, details, t)} aria-label={t("game_page.data_bar")} />}
              acoes={
                <AcoesDoJogo
                  compacto
                  estado={heroState(game, details, phase)}
                  favorito={game.favorite}
                  labels={{
                    jogar: t("game.play"),
                    abrindo: session.launchSlow ? t("desktop.opening_slow") : t("game_page.opening"),
                    voltarAoJogo: t("game.resume"),
                    encerrar: t("desktop.end_game"),
                    instalar: t("desktop.install.label"),
                    emBreve: t("game.upcoming"),
                    favoritar: t("details.favorite"),
                    desfavoritar: t("details.unfavorite"),
                    gerenciar: t("game_page.manage"),
                    configuracoes: t("details.game_settings"),
                  }}
                  onJogar={() => void play(game)}
                  onVoltarAoJogo={() => {
                    void session.resume();
                  }}
                  onAlternarFavorito={() => {}}
                />
              }
            />
          </div>
          {shelves.map((shelf) => (
            <FileiraDeCapas
              key={shelf.spec.id}
              className={styles["fileira"]}
              title={shelfTitle(shelf, t)}
              // O total de continuar jogando e de recentes inclui jogos nunca jogados; só a coleção tem contagem certa.
              count={shelf.spec.kind === "collection" ? t("common.games", { n: shelf.total }) : undefined}
              covers={covers(shelf)}
              previousLabel={t("common.previous")}
              nextLabel={t("common.next")}
              onOpen={onOpenGame}
              onHighlight={highlight}
              onNearEnd={() => {
                overview.loadMore(shelf.spec.id);
              }}
            />
          ))}
        </>
      )}
      <Aviso
        open={notice.open}
        tone={notice.tone}
        noticeKey={notice.key}
        onClose={() => {
          setNotice((n) => ({ ...n, open: false }));
        }}
      >
        {notice.text}
      </Aviso>
    </div>
  );
}

/** O que o primário do destaque faz, com o que já se sabe: sem detalhes, instalado joga e o resto vira Instalar. */
function heroState(game: LibraryGame, details: GameDetails | null, phase: GamePhase): EstadoDoPrimario {
  if (phase === "launching") return "abrindo";
  if (phase !== "none") return "jogando";
  const state = details ? primaryState(details) : game.installed ? "play" : "not-installed";
  return state === "upcoming" ? "em-breve" : state === "not-installed" ? "nao-instalado" : "jogar";
}

/** EstadoCarregando: a estrutura inteira com blocos de espera nos tamanhos do que vai chegar. */
function Esqueleto({ label }: { label: string }) {
  return (
    <div className={styles["esqueleto"]} role="status" aria-label={label}>
      <div className={cx(styles["bloco"], styles["blocoDestaque"])} />
      {[0, 1].map((row) => (
        <div key={row} className={styles["fileira"]}>
          <div className={cx(styles["bloco"], styles["blocoTitulo"])} />
          <div className={styles["blocoCapas"]}>
            {[0, 1, 2, 3, 4].map((col) => (
              <div key={col} className={cx(styles["bloco"], styles["blocoCapa"])} />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
