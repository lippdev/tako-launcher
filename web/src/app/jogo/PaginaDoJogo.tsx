import { useEffect, useMemo, useRef, useState } from "react";
import { CallError, call } from "../../bridge";
import type { GameDetails } from "../../bridge/generated/GameDetails";
import { AcoesDoJogo, type EstadoDoPrimario } from "../../components/AcoesDoJogo/AcoesDoJogo";
import type { MenuEntry } from "../../components/MenuDeContexto/MenuDeContexto";
import { Aviso, type AvisoTone } from "../../components/Aviso/Aviso";
import { CartaoDetalhes, type DetalheDoJogo } from "../../components/CartaoDetalhes/CartaoDetalhes";
import { BarraDeDados, ListaDeSessoes, type DadoDaBarra } from "../../components/CartaoSessoes/CartaoSessoes";
import { CartaoSobre } from "../../components/CartaoSobre/CartaoSobre";
import { Destaque } from "../../components/Destaque/Destaque";
import { FundoDoJogo } from "../../components/FundoDoJogo/FundoDoJogo";
import { PilulaDaLoja } from "../../components/PilulaDaLoja/PilulaDaLoja";
import { StatusDoJogo, type GameStatus as StatusDoChip } from "../../components/StatusDoJogo/StatusDoJogo";
import { TituloDoJogo } from "../../components/TituloDoJogo/TituloDoJogo";
import { useLatest } from "../../lib/useLatest";
import { editGameStrings, gameStrings as s, libraryStrings } from "../strings";
import { DialogoEditarJogo } from "../editar/DialogoEditarJogo";
import { DialogoFontesEIds } from "../editar/DialogoFontesEIds";
import { DialogoRemoverJogo } from "../editar/DialogoRemoverJogo";
import { EtiquetaDeSituacao } from "./EtiquetaDeSituacao";
import { EditorDasEtiquetas } from "./EditorDasEtiquetas";
import { PainelConfiguracoesDoJogo } from "./PainelConfiguracoesDoJogo";
import {
  chipToStatus,
  gameBlocks,
  imageUrl,
  metacriticTone,
  metacriticVerdict,
  primaryState,
  statusToChip,
} from "./jogoModel";
import { useDevImages } from "./useDevImages";
import { useGameDetails } from "./useGameDetails";
import styles from "./PaginaDoJogo.module.css";

interface PaginaDoJogoProps {
  gameId: string;
  /** O núcleo não conhece mais este jogo (saiu da biblioteca): quem abriu a página volta para a Visão geral. */
  onMissing: () => void;
  /**
   * O jogo está rodando agora (EstadoJogando: Voltar ao jogo e Encerrar). A ponte ainda não informa isso, então
   * hoje nada passa `true`; a página já fica pronta para o dia em que o núcleo mandar.
   */
  running?: boolean;
}

interface Notice {
  open: boolean;
  tone: AvisoTone;
  text: string;
  key: number;
}

const CHIP_LABELS: Record<StatusDoChip, string> = s.status;

/** Espera as transições das partes do destaque (e do corpo) acabarem; sem transição (reduzir animações), já resolve. */
function settleParts(root: HTMLElement | null): Promise<unknown> {
  if (!root) return Promise.resolve();
  const animations = [...root.querySelectorAll("[data-parte]")].flatMap((part) => part.getAnimations());
  return Promise.allSettled(animations.map((animation) => animation.finished));
}

/**
 * A página do jogo da biblioteca (README da TelaJogo e da TelaJogoRolada): o destaque e, rolando, os cartões de
 * baixo (sobre, detalhes, sessões). Trocar de jogo não recarrega a página: as partes saem, a página volta ao topo
 * e elas voltam em escada (TrocaDeJogo). O que o núcleo não respondeu não vira bloco.
 */
export function PaginaDoJogo({ gameId, onMissing, running = false }: PaginaDoJogoProps) {
  const { details, error, loading, refresh } = useGameDetails(gameId);
  const [shown, setShown] = useState<GameDetails | null>(null);
  const [entered, setEntered] = useState(false);
  const [opening, setOpening] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  /** Qual diálogo de "Gerenciar" está aberto (um só por vez). */
  const [dialog, setDialog] = useState<"edit" | "ids" | "remove" | null>(null);
  const [notice, setNotice] = useState<Notice>({ open: false, tone: "info", text: "", key: 0 });
  const scrollRef = useRef<HTMLDivElement>(null);
  const latestOnMissing = useLatest(onMissing);
  const devImage = useDevImages();

  // Dado novo do mesmo jogo entra no lugar; de outro jogo, só depois da saída animada (efeito abaixo).
  if (details && shown !== details && (shown === null || shown.game.id === details.game.id)) setShown(details);
  const swapping = details !== null && shown !== null && shown.game.id !== details.game.id;

  useEffect(() => {
    if (!swapping || !details) return;
    let cancelled = false;
    void settleParts(scrollRef.current).then(() => {
      if (cancelled) return;
      scrollRef.current?.scrollTo({ top: 0, behavior: "instant" });
      setShown(details);
    });
    return () => {
      cancelled = true;
    };
  }, [swapping, details]);

  // A primeira vez as partes também entram em escada: começam fora e, dois quadros depois, voltam.
  const ready = shown !== null;
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

  useEffect(() => {
    if (error === "not_found") latestOnMissing.current();
  }, [error, latestOnMissing]);

  const blocks = useMemo(() => (shown ? gameBlocks(shown) : null), [shown]);

  if (!shown || !blocks) {
    return (
      <div className={styles["pagina"]}>
        {error ? (
          <div className={styles["aviso"]} role="alert">
            {s.loadError}
            <button type="button" className={styles["retry"]} onClick={refresh}>
              {libraryStrings.retry}
            </button>
          </div>
        ) : (
          loading && (
            <div className={styles["aviso"]} role="status">
              {s.loading}
            </div>
          )
        )}
      </div>
    );
  }

  const { game } = shown;
  const showNotice = (tone: AvisoTone, text: string) => {
    setNotice({ open: true, tone, text, key: Date.now() });
  };
  /** Chama a ponte; o `library.changed` do núcleo recarrega os detalhes, e o `refresh` garante sem esperar o evento. */
  const change = async (action: () => Promise<unknown>) => {
    try {
      await action();
      refresh();
    } catch (failure) {
      console.error("página do jogo", failure);
      showNotice("error", s.notices.updateError);
    }
  };
  const play = async () => {
    if (opening) return;
    setOpening(true);
    try {
      await call("game.play", { gameId: game.id });
      showNotice("tako", s.notices.opening(game.title));
    } catch (failure) {
      const conflict = failure instanceof CallError && failure.error.code === "conflict";
      if (!conflict) console.error("game.play", failure);
      // `conflict`: o núcleo diz que não dá para jogar (o jogo saiu do disco, ou ainda não saiu). Relê os detalhes.
      showNotice("error", conflict ? s.notices.cannotPlay : s.notices.playError);
      if (conflict) refresh();
    } finally {
      setOpening(false);
    }
  };

  // Editar e remover só existem para jogo adicionado à mão; Fontes e IDs vale para todos (como no Slint).
  const manageEntries: MenuEntry[] = [
    {
      id: "source-ids",
      label: editGameStrings.menu.sourceIds,
      icon: "search",
      onSelect: () => {
        setDialog("ids");
      },
    },
    ...(game.store === "manual"
      ? ([
          {
            id: "edit",
            label: editGameStrings.menu.edit,
            icon: "gear",
            onSelect: () => {
              setDialog("edit");
            },
          },
          { type: "separator" },
          {
            id: "remove",
            label: editGameStrings.menu.remove,
            icon: "trash",
            danger: true,
            onSelect: () => {
              setDialog("remove");
            },
          },
        ] satisfies MenuEntry[])
      : []),
  ];

  const primary = primaryState(shown);
  const state: EstadoDoPrimario = running
    ? "jogando"
    : opening
      ? "abrindo"
      : primary === "upcoming"
        ? "em-breve"
        : primary === "not-installed"
          ? "nao-instalado"
          : "jogar";

  const dataBar: DadoDaBarra[] = [
    ...(blocks.lastPlayed
      ? [{ id: "last", icone: "calendar", rotulo: s.lastSession, valor: blocks.lastPlayed } as const]
      : []),
    ...(blocks.playTime ? [{ id: "time", icone: "clock", rotulo: s.playTime, valor: blocks.playTime } as const] : []),
    ...(blocks.metacritic !== null
      ? [
          {
            id: "metacritic",
            icone: "badge",
            rotulo: s.metacritic,
            valor: s.verdicts[metacriticVerdict(blocks.metacritic)],
            nota: { valor: blocks.metacritic, faixa: metacriticTone(blocks.metacritic) },
          } as const,
        ]
      : []),
  ];

  const detailRows: DetalheDoJogo[] = [
    ...(blocks.developer ? [{ id: "developer", rotulo: s.developer, valor: blocks.developer }] : []),
    ...(blocks.publisher ? [{ id: "publisher", rotulo: s.publisher, valor: blocks.publisher }] : []),
    ...(blocks.release ? [{ id: "release", rotulo: s.release, valor: blocks.release }] : []),
    ...(blocks.genres.length > 0 ? [{ id: "genres", rotulo: s.genres, valor: blocks.genres.join(", ") }] : []),
    ...(blocks.size ? [{ id: "size", rotulo: s.size, valor: blocks.size }] : []),
    {
      id: "installation",
      rotulo: s.installation,
      valor: game.installed ? s.installed : s.notInstalled,
      ligado: game.installed,
    },
  ];

  const leaving = swapping || !entered;
  const hasAbout = blocks.description.length > 0;
  const hasSide = detailRows.length > 0 || blocks.tags.length > 0 || blocks.sessions.length > 0;

  return (
    <div ref={scrollRef} className={styles["pagina"]}>
      <Destaque
        aria-label={s.label(game.title)}
        trocando={leaving}
        fundo={
          <FundoDoJogo
            id={game.id}
            temArte={game.hasHero}
            corDestaque={game.accent}
            src={devImage?.("hero", game.id, game.title)}
          />
        }
        chips={
          <>
            <StatusDoJogo
              status={statusToChip(game.status)}
              rotulos={CHIP_LABELS}
              rotuloDefinir={s.status.set}
              tituloMenu={s.status.label}
              onAlterarStatus={(chip: StatusDoChip) =>
                void change(() => call("game.status.set", { gameIds: [game.id], status: chipToStatus(chip) }))
              }
            />
            {shown.platform && <PilulaDaLoja rotulo={shown.platform} icone="monitor" aria-label={`${s.platform}: ${shown.platform}`} />}
            <PilulaDaLoja rotulo={libraryStrings.stores[game.store]} />
          </>
        }
        situacao={
          running ? (
            <EtiquetaDeSituacao texto={s.running} ligado />
          ) : shown.upcoming ? (
            <EtiquetaDeSituacao texto={s.upcoming} />
          ) : (
            <EtiquetaDeSituacao
              texto={game.installed ? s.installed : s.notInstalled}
              ligado={game.installed}
            />
          )
        }
        titulo={
          <TituloDoJogo
            id={game.id}
            titulo={game.title}
            temLogo={game.hasLogo}
            src={devImage?.("logo", game.id, game.title)}
          />
        }
        descricao={blocks.description[0]}
        dados={<BarraDeDados dados={dataBar} aria-label={s.dataBar} />}
        acoes={
          <AcoesDoJogo
            estado={state}
            favorito={game.favorite}
            rotulos={{
              jogar: s.actions.play,
              abrindo: s.actions.opening,
              voltarAoJogo: s.actions.resume,
              encerrar: s.actions.stop,
              instalar: s.actions.install,
              emBreve: s.actions.upcoming,
              favoritar: s.actions.favorite,
              desfavoritar: s.actions.unfavorite,
              gerenciar: s.actions.manage,
              configuracoes: s.actions.settings,
            }}
            onJogar={() => void play()}
            onAlternarFavorito={() =>
              void change(() => call("game.favorite.set", { gameIds: [game.id], favorite: !game.favorite }))
            }
            onConfiguracoes={() => {
              setSettingsOpen(true);
            }}
            entradasExtras={manageEntries}
          />
        }
      />
      {(hasAbout || hasSide) && (
        <div className={styles["corpo"]} data-parte="" data-trocando={leaving}>
          <div className={styles["grade"]}>
            {hasAbout && <CartaoSobre titulo={s.about} paragrafos={blocks.description} className={styles["sobre"]} />}
            {hasSide && (
              <div className={styles["lateral"]}>
                <CartaoDetalhes
                  aria-label={s.details}
                  linhas={detailRows}
                  etiquetas={blocks.tags}
                  rotuloEtiquetas={s.tags}
                  editorDeEtiquetas={
                    <EditorDasEtiquetas
                      gameId={game.id}
                      tags={blocks.tags}
                      onChanged={refresh}
                      onError={(text) => {
                        showNotice("error", text);
                      }}
                    />
                  }
                />
                <ListaDeSessoes titulo={s.recentSessions} sessoes={blocks.sessions.map((x) => ({ inicio: x.startedAt, duracao: x.duration }))} />
              </div>
            )}
          </div>
        </div>
      )}
      <PainelConfiguracoesDoJogo
        key={game.id}
        gameId={game.id}
        title={game.title}
        coverUrl={
          game.hasCover
            ? (devImage?.("cover", game.id, game.title) ??
              imageUrl("cover", game.id, 36 * window.devicePixelRatio, 46 * window.devicePixelRatio))
            : undefined
        }
        open={settingsOpen}
        onClose={() => {
          setSettingsOpen(false);
        }}
        onError={(text) => {
          showNotice("error", text);
        }}
      />
      <DialogoEditarJogo
        key={`edit-${game.id}`}
        gameId={game.id}
        open={dialog === "edit"}
        onClose={() => {
          setDialog(null);
        }}
        onSaved={(title) => {
          setDialog(null);
          showNotice("success", editGameStrings.saved(title));
          refresh();
        }}
        onError={(text) => {
          showNotice("error", text);
        }}
      />
      <DialogoFontesEIds
        key={`ids-${game.id}`}
        gameId={game.id}
        open={dialog === "ids"}
        onClose={() => {
          setDialog(null);
        }}
        onNotice={showNotice}
      />
      <DialogoRemoverJogo
        key={`remove-${game.id}`}
        gameId={game.id}
        title={game.title}
        open={dialog === "remove"}
        onClose={() => {
          setDialog(null);
        }}
        onRemoved={() => {
          setDialog(null);
          // O núcleo já não conhece o jogo: o `library.changed` faz a leitura dar `not_found` e a página volta à Visão geral.
          refresh();
        }}
        onError={(text) => {
          showNotice("error", text);
        }}
      />
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
