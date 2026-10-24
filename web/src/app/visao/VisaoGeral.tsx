import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import { CallError, call } from "../../bridge";
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
import { gameStrings as g, libraryStrings, overviewStrings as s } from "../strings";
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

function shelfTitle(shelf: ShelfState): string {
  switch (shelf.spec.id) {
    case CONTINUE_ID:
      return s.continuePlaying;
    case RECENT_ID:
      return s.recent;
    default:
      return shelf.spec.name ?? "";
  }
}

/** Dados embaixo à esquerda do destaque: o que a lista já sabe, mais a nota quando os detalhes chegaram. */
function heroData(game: LibraryGame, details: GameDetails | null): DadoDaBarra[] {
  const last = formatLastPlayed(game.lastPlayedAt);
  const time = formatPlayTime(game.playTimeSeconds);
  const score = validScore(details?.metacriticScore ?? null);
  return [
    ...(last ? [{ id: "last", icone: "calendar", rotulo: g.lastSession, valor: last } as const] : []),
    ...(time ? [{ id: "time", icone: "clock", rotulo: g.playTime, valor: time } as const] : []),
    ...(score !== null
      ? [
          {
            id: "metacritic",
            icone: "badge",
            rotulo: g.metacritic,
            valor: g.verdicts[metacriticVerdict(score)],
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
  const devImage = useDevImages();
  const heroRef = useRef<HTMLDivElement>(null);
  const [focusId, setFocusId] = useState<string | null>(null);
  const [shownId, setShownId] = useState<string | null>(null);
  const [entered, setEntered] = useState(false);
  const [opening, setOpening] = useState(false);
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
    if (opening) return;
    setOpening(true);
    try {
      await call("game.play", { gameId: game.id });
      showNotice("tako", g.notices.opening(game.title));
    } catch (failure) {
      const conflict = failure instanceof CallError && failure.error.code === "conflict";
      if (!conflict) console.error("game.play", failure);
      showNotice("error", conflict ? g.notices.cannotPlay : g.notices.playError);
    } finally {
      setOpening(false);
    }
  };

  const game = shownId !== null ? games.get(shownId) : undefined;
  const details = shownId !== null ? (entries[shownId]?.details ?? null) : null;
  const covers = (shelf: ShelfState): CapaDaFileira[] =>
    shelf.games.map((item) => {
      const time = formatPlayTime(item.playTimeSeconds);
      const src = devImage?.("cover", item.id, item.title);
      return {
        id: item.id,
        titulo: item.title,
        rotulo: time ? s.playedFor(time) : null,
        corDestaque: item.accent,
        temArte: item.hasCover,
        src,
      };
    });

  const empty = overview.status === "ready" && overview.total === 0;
  const headCount =
    overview.status === "loading" ? s.loading : overview.status === "ready" ? libraryStrings.count(overview.total) : null;

  const openHero = (event: MouseEvent<HTMLDivElement>) => {
    // Clicar numa ação (Jogar) ou no menu de uma camada não abre a página; o resto do destaque abre.
    const target = event.target as Element;
    if (!event.currentTarget.contains(target) || target.closest("button, a, [role='menu']")) return;
    if (game) onOpenGame(game.id);
  };

  return (
    <div className={styles["pagina"]}>
      <div className={styles["cabeca"]}>
        <h1 className="page-title">{s.welcome}</h1>
        {headCount && <span className={styles["total"]}>{headCount}</span>}
      </div>

      {overview.status === "error" ? (
        <div className={styles["aviso"]} role="alert">
          {s.loadError}
          <button type="button" className={styles["retry"]} onClick={overview.retry}>
            {s.retry}
          </button>
        </div>
      ) : empty ? (
        <div className={styles["vazia"]}>
          <img className={styles["polvo"]} src={mark} alt="" />
          <p>{s.empty}</p>
        </div>
      ) : !game ? (
        <Esqueleto label={s.loading} />
      ) : (
        <>
          {/* O clique no destaque é um atalho do mouse; pelo teclado o jogo se abre pela capa ou pela lista. */}
          <div ref={heroRef} className={styles["destaque"]} onClick={openHero}>
            <Destaque
              variante="visao"
              aria-label={s.heroLabel(game.title)}
              trocando={swapping || !entered}
              fundo={
                <FundoDoJogo
                  id={game.id}
                  temArte={game.hasHero}
                  corDestaque={game.accent}
                  src={devImage?.("hero", game.id, game.title)}
                />
              }
              chips={<PilulaDaLoja rotulo={libraryStrings.stores[game.store]} />}
              situacao={
                game.installed && game.lastPlayedAt !== null ? (
                  <EtiquetaDeSituacao texto={s.continuePlaying} ligado />
                ) : (
                  <EtiquetaDeSituacao
                    texto={game.installed ? g.installed : g.notInstalled}
                    ligado={game.installed}
                  />
                )
              }
              titulo={
                <TituloDoJogo
                  id={game.id}
                  titulo={game.title}
                  temLogo={game.hasLogo}
                  tamanho="visao"
                  src={devImage?.("logo", game.id, game.title)}
                />
              }
              descricao={paragraphs(details?.description ?? null)[0]}
              dados={<BarraDeDados dados={heroData(game, details)} aria-label={g.dataBar} />}
              acoes={
                <AcoesDoJogo
                  compacto
                  estado={heroState(game, details, opening)}
                  favorito={game.favorite}
                  rotulos={{
                    jogar: g.actions.play,
                    abrindo: g.actions.opening,
                    voltarAoJogo: g.actions.resume,
                    encerrar: g.actions.stop,
                    instalar: g.actions.install,
                    emBreve: g.actions.upcoming,
                    favoritar: g.actions.favorite,
                    desfavoritar: g.actions.unfavorite,
                    gerenciar: g.actions.manage,
                    configuracoes: g.actions.settings,
                  }}
                  onJogar={() => void play(game)}
                  onAlternarFavorito={() => {}}
                />
              }
            />
          </div>
          {shelves.map((shelf) => (
            <FileiraDeCapas
              key={shelf.spec.id}
              className={styles["fileira"]}
              titulo={shelfTitle(shelf)}
              // O total de continuar jogando e de recentes inclui jogos nunca jogados; só a coleção tem contagem certa.
              contagem={shelf.spec.kind === "collection" ? libraryStrings.count(shelf.total) : undefined}
              capas={covers(shelf)}
              rotuloAnterior={s.previous}
              rotuloProximo={s.next}
              onAbrir={onOpenGame}
              onDestacar={highlight}
              onPertoDoFim={() => {
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
function heroState(game: LibraryGame, details: GameDetails | null, opening: boolean): EstadoDoPrimario {
  if (opening) return "abrindo";
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
          <div className={styles["blocosCapas"]}>
            {Array.from({ length: 5 }, (_, i) => (
              <div key={i}>
                <div className={cx(styles["bloco"], styles["blocoCapa"])} />
                <div className={cx(styles["bloco"], styles["blocoNome"])} />
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
