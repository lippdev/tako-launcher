import {
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";
import { Portal } from "../../lib/Portal";
import { cx } from "../../lib/cx";
import { useEscapeLayer } from "../../lib/escapeLayers";
import { useFocusTrap } from "../../lib/focusTrap";
import { placeMenu, rootZoom, type Anchor } from "../../lib/position";
import { usePresence } from "../../lib/presence";
import { useLatest } from "../../lib/useLatest";
import { Icon } from "../Icon/Icon";
import styles from "./PainelDownloads.module.css";

export type EstadoDownload =
  | "baixando"
  | "pausado"
  | "na_fila"
  | "concluido"
  | "erro";

export interface ItemDownload {
  id: string;
  title: string;
  /** Progresso de 0 a 100 ou null (quando null, a barra não é exibida). */
  progress?: number | null;
  /** Velocidade como texto pronto ou null (ex.: "12 MB/s"). */
  speed?: string | null;
  /** Tempo restante como texto pronto ou null (ex.: "15 min"). */
  remainingTime?: string | null;
  /** Tamanho como texto pronto ou null (ex.: "60.9 / 108.7 GB"). */
  size?: string | null;
  /** Estado atual do item. */
  state:
    | EstadoDownload
    | "na fila"
    | "concluído"
    | "downloading"
    | "paused"
    | "queued"
    | "completed"
    | "error";
  /** URL da imagem da capa (opcional; 36×46). */
  cover?: string | null;
  /** Mensagem descritiva de erro quando estado === "erro". */
  error?: string | null;
  /** Subtítulo customizado opcional (se omitido, montado a partir dos campos presentes). */
  subtitle?: string | null;
}

export type PainelAnchor =
  | HTMLElement
  | RefObject<HTMLElement | null>
  | { x: number; y: number }
  | null;

export interface PainelDownloadsProps {
  /** Se o painel está aberto. */
  open?: boolean;
  /** Callback para fechar o painel (acionado por Esc ou clique fora). */
  onClose?: () => void;
  /** Elemento, ref ou coordenadas de ancoragem para posicionar o popover (padrão: alinhado à direita do botão). */
  anchor?: PainelAnchor;
  /** Lista única de todos os downloads (agrupados automaticamente em seções). */
  items?: ItemDownload[];
  /** Ou listas separadas por categoria: */
  downloading?: ItemDownload[];
  queued?: ItemDownload[];
  completed?: ItemDownload[];
  /** Se deve renderizar inline (sem Portal nem posicionamento fixo), útil para visualização estática na galeria. */
  inline?: boolean;
  /** Callbacks de ação para itens: */
  onPause?: (id: string) => void;
  onResume?: (id: string) => void;
  onCancel?: (id: string) => void;
  onOpenGame?: (id: string) => void;
  /** Textos customizáveis recebidos por props: */
  title?: string;
  activeLabel?: string | ((count: number) => string);
  emptyLabel?: string;
  sectionLabels?: {
    downloading?: string;
    queued?: string;
    completed?: string;
  };
  actionLabels?: {
    pause?: string;
    resume?: string;
    cancel?: string;
    openGame?: string;
  };
  stateLabels?: Partial<Record<EstadoDownload, string>>;
  className?: string;
  "aria-label"?: string;
  children?: ReactNode;
}

function normalizarEstado(
  estado: ItemDownload["state"]
): EstadoDownload {
  switch (estado) {
    case "pausado":
    case "paused":
      return "pausado";
    case "na_fila":
    case "na fila":
    case "queued":
      return "na_fila";
    case "concluido":
    case "concluído":
    case "completed":
      return "concluido";
    case "erro":
    case "error":
      return "erro";
    case "baixando":
    case "downloading":
    default:
      return "baixando";
  }
}

function normalizarItem(item: ItemDownload): ItemDownload & { estadoNormalizado: EstadoDownload } {
  return {
    ...item,
    id: item.id,
    title: item.title ?? "",
    progress: item.progress ?? null,
    speed: item.speed ?? null,
    remainingTime: item.remainingTime ?? null,
    size: item.size ?? null,
    cover: item.cover ?? null,
    error: item.error ?? null,
    subtitle: item.subtitle ?? null,
    estadoNormalizado: normalizarEstado(item.state),
  };
}

function formatarSubtitulo(
  item: ReturnType<typeof normalizarItem>,
  rotulosEstado: Record<EstadoDownload, string>
): string {
  if (item.subtitle) return item.subtitle;

  const estadoTexto =
    item.estadoNormalizado === "erro" && item.error
      ? item.error
      : rotulosEstado[item.estadoNormalizado];

  const partes = [
    estadoTexto,
    item.size,
    item.speed,
    item.remainingTime,
  ].filter((p): p is string => Boolean(p && p.trim().length > 0));

  return partes.join(" · ");
}

/**
 * PainelDownloads: popover com as listas de Baixando, Na fila e Concluídos.
 * Abre como camada com Portal, Esc fecha, foco preso dentro e devolvido ao abrir.
 */
export function PainelDownloads(props: PainelDownloadsProps) {
  const {
    open = false,
    onClose,
    anchor,
    items,
    downloading,
    queued,
    completed,
    inline = false,
    onPause,
    onResume,
    onCancel,
    onOpenGame,
    title = "Downloads",
    activeLabel,
    emptyLabel = "Nenhum download ativo",
    sectionLabels,
    actionLabels,
    stateLabels: rotulosEstadoProp,
    className,
    "aria-label": ariaLabelProp,
    children,
  } = props;

  const estaAberto = inline || open;
  const fechar = onClose;
  const fecharRef = useLatest(fechar);

  const panelRef = useRef<HTMLDivElement>(null);
  const titleId = useId();
  // Capa que não carregou (o jogo ainda não tem arte no cache): fica o quadrado do estado, sem imagem quebrada.
  const [brokenCovers, setBrokenCovers] = useState<ReadonlySet<string>>(new Set());

  // Camada: presença para animação suave de saída (200ms)
  const { mounted, visible, animating } = usePresence(estaAberto, panelRef);

  // Esc e foco preso quando em modo camada
  useEscapeLayer(!inline && estaAberto, () => {
    fecharRef.current?.();
  });
  useFocusTrap(!inline && estaAberto, panelRef);

  // Clique fora fecha o popover (o botão de ancoragem é preservado)
  useEffect(() => {
    if (inline || !estaAberto) return;
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target instanceof Node ? event.target : null;
      if (!target || panelRef.current?.contains(target)) return;

      const resolvedAnchor =
        anchor && "current" in anchor ? anchor.current : anchor;

      if (resolvedAnchor instanceof HTMLElement && resolvedAnchor.contains(target)) {
        return;
      }
      fecharRef.current?.();
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
    };
  }, [inline, estaAberto, anchor, fecharRef]);

  // Posicionamento dinâmico relativo à âncora (6px abaixo, alinhado à direita)
  useLayoutEffect(() => {
    if (inline || !mounted || !panelRef.current) return;
    const panel = panelRef.current;

    const place = () => {
      const zoom = rootZoom();
      const margin = 8 * zoom;
      const viewport = { width: window.innerWidth, height: window.innerHeight };

      const resolvedAnchor =
        anchor && "current" in anchor ? anchor.current : anchor;

      if (resolvedAnchor) {
        const rect: Anchor =
          resolvedAnchor instanceof HTMLElement
            ? resolvedAnchor.getBoundingClientRect()
            : { left: resolvedAnchor.x, right: resolvedAnchor.x, top: resolvedAnchor.y, bottom: resolvedAnchor.y };

        const natural = {
          width: panel.offsetWidth * zoom,
          height: panel.offsetHeight * zoom,
        };

        const spot = placeMenu(rect, natural, viewport, {
          align: "end",
          gap: 6 * zoom,
          margin,
        });

        panel.style.left = `${spot.left / zoom}px`;
        panel.style.top = `${spot.top / zoom}px`;
        panel.style.right = "";
      } else {
        // Sem âncora: posicionamento padrão no canto superior direito
        panel.style.top = `${58 / zoom}px`;
        panel.style.right = `${24 / zoom}px`;
        panel.style.left = "";
      }
    };

    place();
    window.addEventListener("resize", place);
    window.addEventListener("scroll", place, true);
    return () => {
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", place, true);
    };
  }, [inline, mounted, anchor]);

  if (!inline && !mounted) return null;

  // Unificação e separação das listas de itens
  const todosItens = (items ?? []).map(normalizarItem);
  const listaBaixando = (downloading ?? []).map(normalizarItem);
  const listaNaFila = (queued ?? []).map(normalizarItem);
  const listaConcluidos = (completed ?? []).map(normalizarItem);

  const secoesSeparadas =
    listaBaixando.length > 0 || listaNaFila.length > 0 || listaConcluidos.length > 0;

  const grupos = secoesSeparadas
    ? [
        { chave: "downloading", title: sectionLabels?.downloading ?? "Baixando", items: listaBaixando },
        { chave: "queued", title: sectionLabels?.queued ?? "Na fila", items: listaNaFila },
        { chave: "completed", title: sectionLabels?.completed ?? "Concluídos", items: listaConcluidos },
      ].filter((g) => g.items.length > 0)
    : [
        {
          chave: "todos",
          title: "",
          items: todosItens,
        },
      ];

  const totalGeral = secoesSeparadas
    ? listaBaixando.length + listaNaFila.length + listaConcluidos.length
    : todosItens.length;

  const totalAtivos = secoesSeparadas
    ? listaBaixando.length + listaNaFila.length
    : todosItens.filter((i) => i.estadoNormalizado !== "concluido").length;

  // Rótulos padrão de estados e ações
  const rotulosEstado: Record<EstadoDownload, string> = {
    baixando: "Baixando",
    pausado: "Pausado",
    na_fila: "Na fila",
    concluido: "Concluído",
    erro: "Erro",
    ...rotulosEstadoProp,
  };

  const rotulosAcoesFinais = {
    pause: "Pausar",
    resume: "Retomar",
    cancel: "Cancelar",
    openGame: "Abrir o jogo",
    ...actionLabels,
  };

  const rotuloAtivosTexto =
    totalAtivos > 0
      ? typeof activeLabel === "function"
        ? activeLabel(totalAtivos)
        : activeLabel ?? `${totalAtivos} ${totalAtivos === 1 ? "ativo" : "ativos"}`
      : null;

  const conteudo = (
    <div
      ref={panelRef}
      role="dialog"
      aria-modal={!inline}
      aria-labelledby={titleId}
      aria-label={ariaLabelProp ?? title}
      tabIndex={-1}
      data-visible={inline || visible}
      data-animating={!inline && animating}
      className={cx(
        styles["popover"],
        inline && styles["inline"],
        className
      )}
    >
      <header className={styles["header"]}>
        <h6 className={styles["headerTitle"]} id={titleId}>
          <span>{title}</span>
          {rotuloAtivosTexto && <span>{rotuloAtivosTexto}</span>}
        </h6>
      </header>

      {totalGeral === 0 ? (
        <div className={styles["empty"]}>{emptyLabel}</div>
      ) : (
        <div className={styles["list"]} role="list">
          {grupos.map((grupo, gIdx) => (
            <div key={grupo.chave}>
              {gIdx > 0 && <hr className={styles["separator"]} />}
              {grupo.title && secoesSeparadas && (
                <div className={styles["sectionHeader"]}>{grupo.title}</div>
              )}
              {grupo.items.map((item) => {
                const subtitulo = formatarSubtitulo(item, rotulosEstado);
                const progressoValido =
                  item.progress != null
                    ? Math.max(0, Math.min(100, item.progress))
                    : null;

                const podePausar = item.estadoNormalizado === "baixando";
                const podeRetomar =
                  item.estadoNormalizado === "pausado" || item.estadoNormalizado === "erro";
                const podeAbrir = item.estadoNormalizado === "concluido";
                const podeCancelar =
                  Boolean(onCancel) && item.estadoNormalizado !== "concluido";

                return (
                  <div key={item.id} className={styles["item"]} role="listitem">
                    {item.cover && !brokenCovers.has(item.cover) ? (
                      <img
                        src={item.cover}
                        alt=""
                        className={styles["cover"]}
                        onError={() => {
                          setBrokenCovers((previous) => new Set(previous).add(item.cover ?? ""));
                        }}
                      />
                    ) : (
                      <div
                        className={cx(
                          styles["coverPlaceholder"],
                          item.estadoNormalizado === "baixando" && styles["spinner"]
                        )}
                        aria-hidden="true"
                      >
                        {item.estadoNormalizado === "baixando" ? (
                          <Icon name="refresh" size="sm" />
                        ) : (
                          <Icon name="download" size="sm" />
                        )}
                      </div>
                    )}

                    <div className={styles["meta"]}>
                      <strong className={styles["itemTitle"]} title={item.title}>
                        {item.title}
                      </strong>
                      {subtitulo && <small className={styles["itemSub"]}>{subtitulo}</small>}
                      {progressoValido != null && (
                        <div
                          className={styles["bar"]}
                          role="progressbar"
                          aria-valuenow={progressoValido}
                          aria-valuemin={0}
                          aria-valuemax={100}
                          aria-label={`Progresso de ${item.title}: ${progressoValido}%`}
                        >
                          <b
                            className={styles["barFill"]}
                            style={{ transform: `scaleX(${progressoValido / 100})` }}
                          />
                        </div>
                      )}
                    </div>

                    <div className={styles["actions"]}>
                      {podePausar && (
                        <button
                          type="button"
                          className={styles["actionBtn"]}
                          onClick={() => onPause?.(item.id)}
                          aria-label={`${rotulosAcoesFinais.pause} ${item.title}`}
                          title={rotulosAcoesFinais.pause}
                        >
                          <Icon name="pause" size="sm" />
                        </button>
                      )}
                      {podeRetomar && (
                        <button
                          type="button"
                          className={styles["actionBtn"]}
                          onClick={() => onResume?.(item.id)}
                          aria-label={`${rotulosAcoesFinais.resume} ${item.title}`}
                          title={rotulosAcoesFinais.resume}
                        >
                          <Icon
                            name={item.estadoNormalizado === "erro" ? "refresh" : "play"}
                            size="sm"
                            filled={item.estadoNormalizado !== "erro"}
                          />
                        </button>
                      )}
                      {podeAbrir && (
                        <button
                          type="button"
                          className={styles["actionBtn"]}
                          onClick={() => onOpenGame?.(item.id)}
                          aria-label={`${rotulosAcoesFinais.openGame} ${item.title}`}
                          title={rotulosAcoesFinais.openGame}
                        >
                          <Icon name="play" size="sm" filled />
                        </button>
                      )}
                      {podeCancelar && (
                        <button
                          type="button"
                          className={styles["actionBtn"]}
                          onClick={() => onCancel?.(item.id)}
                          aria-label={`${rotulosAcoesFinais.cancel} ${item.title}`}
                          title={rotulosAcoesFinais.cancel}
                        >
                          <Icon name="x" size="sm" />
                        </button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      )}

      {children}
    </div>
  );

  if (inline) {
    return conteudo;
  }

  return <Portal>{conteudo}</Portal>;
}
