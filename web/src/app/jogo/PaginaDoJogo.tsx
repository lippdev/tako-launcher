import { useEffect, useMemo, useRef, useState } from "react";
import { call } from "../../bridge";
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
import { useT } from "../../i18n";
import { useLatest } from "../../lib/useLatest";
import { DialogoEditarJogo } from "../editar/DialogoEditarJogo";
import { DialogoFontesEIds } from "../editar/DialogoFontesEIds";
import { DialogoRemoverJogo } from "../editar/DialogoRemoverJogo";
import { DialogoSeletorDeImagens } from "./DialogoSeletorDeImagens";
import { useGameSession } from "../sessao/useGameSession";
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
import { COMMAND_EVENT } from "../../focus";
import styles from "./PaginaDoJogo.module.css";

interface PaginaDoJogoProps {
  gameId: string;
  /** O núcleo não conhece mais este jogo (saiu da biblioteca): quem abriu a página volta para a Visão geral. */
  onMissing: () => void;
}

interface Notice {
  open: boolean;
  tone: AvisoTone;
  text: string;
  key: number;
}

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
export function PaginaDoJogo({ gameId, onMissing }: PaginaDoJogoProps) {
  const { details, error, loading, refresh } = useGameDetails(gameId);
  const [shown, setShown] = useState<GameDetails | null>(null);
  const [entered, setEntered] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  /** Qual diálogo de "Gerenciar" está aberto (um só por vez). */
  const [dialog, setDialog] = useState<"edit" | "ids" | "remove" | "art" | null>(null);
  const [notice, setNotice] = useState<Notice>({ open: false, tone: "info", text: "", key: 0 });
  const scrollRef = useRef<HTMLDivElement>(null);
  const latestOnMissing = useLatest(onMissing);
  const devImage = useDevImages();
  const t = useT();
  // A sessão é da interface inteira (abrindo, jogando, fechando); a página só lê o ponto em que este jogo está.
  const session = useGameSession();
  const phase = session.phaseOf(gameId);
  const running = phase !== "none";

  const chipLabels = useMemo<Record<StatusDoChip, string>>(
    () => ({
      none: t("game_page.status_none"),
      playing: t("game_page.status_playing"),
      completed: t("game_page.status_completed"),
      queued: t("game_page.status_queued"),
      paused: t("game_page.status_paused"),
      dropped: t("game_page.status_dropped"),
    }),
    [t],
  );

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

  useEffect(() => {
    const el = scrollRef.current;
    if (!el || !shown) return;
    const { game } = shown;
    const onCommand = (e: Event) => {
      const ce = e as CustomEvent<{ command: "favorite" | "menu" }>;
      if (ce.detail?.command === "favorite") {
        ce.preventDefault();
        void call("game.favorite.set", { gameIds: [game.id], favorite: !game.favorite })
          .then(() => refresh())
          .catch((err) => console.error("página do jogo", err));
      } else if (ce.detail?.command === "menu") {
        // Como no Slint (`Action::More`): o Menu do controle abre o Gerenciar do jogo.
        const manage = el.querySelector<HTMLButtonElement>("[data-command-menu]");
        if (manage) {
          ce.preventDefault();
          manage.click();
        }
      }
    };
    el.addEventListener(COMMAND_EVENT, onCommand);
    return () => el.removeEventListener(COMMAND_EVENT, onCommand);
  }, [shown, refresh]);

  if (!shown || !blocks) {
    return (
      <div className={styles["pagina"]}>
        {error ? (
          <div className={styles["aviso"]} role="alert">
            {t("game_page.load_error")}
            <button type="button" className={styles["retry"]} onClick={refresh}>
              {t("game_page.retry")}
            </button>
          </div>
        ) : (
          loading && (
            <div className={styles["aviso"]} role="status">
              {t("game_page.loading")}
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
      showNotice("error", t("game_page.update_error"));
    }
  };
  const play = async () => {
    if (running) return;
    const feedback = await session.play(game);
    showNotice(feedback.tone, feedback.text);
    // Não abriu (o jogo saiu do disco, ou há outro aberto): relê os detalhes.
    if (feedback.tone === "error") refresh();
  };
  /** Encerrar; se o núcleo recusar de vez, avisa (o `conflict` vira a oferta de forçar no diálogo "Fechando o jogo…"). */
  const stop = async () => {
    if ((await session.end()) === "error") showNotice("error", t("game_page.end_error"));
  };
  const cancellable = phase === "launching" && session.launchSlow;

  // Editar e remover só existem para jogo adicionado à mão; Fontes e IDs vale para todos (como no Slint).
  const manageEntries: MenuEntry[] = [
    {
      id: "change-art",
      label: t("options.change_art"),
      icon: "image",
      onSelect: () => {
        setDialog("art");
      },
    },
    {
      id: "source-ids",
      label: t("options.source_ids"),
      icon: "search",
      onSelect: () => {
        setDialog("ids");
      },
    },
    ...(game.store === "manual"
      ? ([
          {
            id: "edit",
            label: t("options.edit"),
            icon: "gear",
            onSelect: () => {
              setDialog("edit");
            },
          },
          { type: "separator" },
          {
            id: "remove",
            label: t("options.remove"),
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
  const state: EstadoDoPrimario =
    phase === "launching"
      ? "abrindo"
      : running
        ? "jogando"
        : primary === "upcoming"
        ? "em-breve"
        : primary === "not-installed"
          ? "nao-instalado"
          : "jogar";

  const verdictText = (verdict: "universal" | "favorable" | "mixed" | "unfavorable") => {
    switch (verdict) {
      case "universal":
        return t("game_page.verdict_universal");
      case "favorable":
        return t("game_page.verdict_favorable");
      case "mixed":
        return t("game_page.verdict_mixed");
      case "unfavorable":
        return t("game_page.verdict_unfavorable");
    }
  };

  const dataBar: DadoDaBarra[] = [
    ...(blocks.lastPlayed
      ? [{ id: "last", icone: "calendar", label: t("game_page.last_session"), valor: blocks.lastPlayed } as const]
      : []),
    ...(blocks.playTime ? [{ id: "time", icone: "clock", label: t("game_page.play_time"), valor: blocks.playTime } as const] : []),
    ...(blocks.metacritic !== null
      ? [
          {
            id: "metacritic",
            icone: "badge",
            label: t("game_page.metacritic"),
            valor: verdictText(metacriticVerdict(blocks.metacritic)),
            nota: { valor: blocks.metacritic, faixa: metacriticTone(blocks.metacritic) },
          } as const,
        ]
      : []),
  ];

  const detailRows: DetalheDoJogo[] = [
    ...(blocks.developer ? [{ id: "developer", label: t("game_page.developer"), valor: blocks.developer }] : []),
    ...(blocks.publisher ? [{ id: "publisher", label: t("game_page.publisher"), valor: blocks.publisher }] : []),
    ...(blocks.release ? [{ id: "release", label: t("game_page.release"), valor: blocks.release }] : []),
    ...(blocks.genres.length > 0 ? [{ id: "genres", label: t("game_page.genres"), valor: blocks.genres.join(", ") }] : []),
    ...(blocks.size ? [{ id: "size", label: t("game_page.size"), valor: blocks.size }] : []),
    {
      id: "installation",
      label: t("game_page.installation"),
      valor: game.installed ? t("game_page.installed") : t("game_page.not_installed"),
      ligado: game.installed,
    },
  ];

  const leaving = swapping || !entered;
  const hasAbout = blocks.description.length > 0;
  const hasSide = detailRows.length > 0 || blocks.tags.length > 0 || blocks.sessions.length > 0;

  return (
    <div ref={scrollRef} className={styles["pagina"]}>
      <Destaque
        aria-label={t("game_page.label", { title: game.title })}
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
              labels={chipLabels}
              defineLabel={t("game_page.status_set")}
              menuTitle={t("game_page.status_label")}
              onStatusChange={(chip: StatusDoChip) =>
                void change(() => call("game.status.set", { gameIds: [game.id], status: chipToStatus(chip) }))
              }
            />
            {shown.platform && <PilulaDaLoja label={shown.platform} icon="monitor" aria-label={`${t("game_page.platform")}: ${shown.platform}`} />}
            <PilulaDaLoja label={t(`sources.store.${game.store}`)} />
          </>
        }
        situacao={
          running ? (
            <EtiquetaDeSituacao
              texto={session.elapsed ? `${t("game_page.running")} · ${session.elapsed}` : t("game_page.running")}
              ligado
            />
          ) : shown.upcoming ? (
            <EtiquetaDeSituacao texto={t("game_page.upcoming")} />
          ) : (
            <EtiquetaDeSituacao
              texto={game.installed ? t("game_page.installed") : t("game_page.not_installed")}
              ligado={game.installed}
            />
          )
        }
        title={
          <TituloDoJogo
            id={game.id}
            title={game.title}
            hasLogo={game.hasLogo}
            src={devImage?.("logo", game.id, game.title)}
          />
        }
        descricao={blocks.description[0]}
        dados={<BarraDeDados dados={dataBar} aria-label={t("game_page.data_bar")} />}
        acoes={
          <AcoesDoJogo
            estado={state}
            favorito={game.favorite}
            labels={{
              jogar: t("game_page.play"),
              abrindo: session.launchSlow ? t("desktop.opening_slow") : t("game_page.opening"),
              voltarAoJogo: t("game_page.resume"),
              encerrar: cancellable ? t("common.cancel") : t("game_page.stop"),
              instalar: t("game_page.install"),
              emBreve: t("game_page.upcoming"),
              favoritar: t("game_page.favorite"),
              desfavoritar: t("game_page.unfavorite"),
              gerenciar: t("game_page.manage"),
              configuracoes: t("game_page.settings"),
            }}
            onJogar={() => void play()}
            onVoltarAoJogo={() => {
              void session.resume();
            }}
            onEncerrar={
              running && (phase !== "launching" || cancellable)
                ? () => {
                    void stop();
                  }
                : undefined
            }
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
            {hasAbout && <CartaoSobre title={t("game_page.about")} paragrafos={blocks.description} className={styles["sobre"]} />}
            {hasSide && (
              <div className={styles["lateral"]}>
                <CartaoDetalhes
                  aria-label={t("game_page.details")}
                  linhas={detailRows}
                  etiquetas={blocks.tags}
                  rotuloEtiquetas={t("game_page.tags")}
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
                <ListaDeSessoes title={t("game_page.recent_sessions")} sessoes={blocks.sessions.map((x) => ({ inicio: x.startedAt, duracao: x.duration }))} />
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
          showNotice("success", t("edit_game.saved", { game: title }));
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
      <DialogoSeletorDeImagens
        key={`art-${game.id}`}
        gameId={game.id}
        gameTitle={game.title}
        open={dialog === "art"}
        onClose={() => {
          setDialog(null);
        }}
        onImageChanged={() => {
          refresh();
        }}
      />
      <Aviso
        open={notice.open}
        tone={notice.tone}
        key={notice.key}
        onClose={() => {
          setNotice((n) => ({ ...n, open: false }));
        }}
      >
        {notice.text}
      </Aviso>
    </div>
  );
}
