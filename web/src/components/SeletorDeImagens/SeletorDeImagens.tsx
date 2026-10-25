import {
  useState,
  useMemo,
  useRef,
  type KeyboardEvent,
} from "react";
import { Dialogo } from "../Dialogo/Dialogo";
import { Abas, type Aba } from "../Abas/Abas";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { Icon } from "../Icon/Icon";
import { cx } from "../../lib/cx";
import styles from "./SeletorDeImagens.module.css";

export type TipoDeImagem = "capa" | "fundo" | "logo" | "icone";

export interface OpcaoDeImagem {
  id: string;
  url: string;
  /** Nome legível da fonte de onde a imagem foi obtida (ex.: "Steam", "SteamGridDB", "Arquivo local"). */
  source: string;
  /** Dimensões da arte (ex.: "600 × 900", "1920 × 1080"). */
  dimensions?: string;
  /** Estilo da arte para filtros (ex.: "Oficial", "Alternativa"). */
  style?: string;
  /** Indica se esta imagem é a que está atualmente definida para o jogo. */
  current?: boolean;
}

export interface SeletorDeImagensLabels {
  /** Rótulo do botão fechar (acessibilidade / aria-label). */
  close?: string;
  /** Prefixo do título do diálogo (ex.: "Imagens de"). */
  titlePrefix?: string;
  /** Subtítulo descritivo por tipo de imagem. */
  subtitles?: Partial<Record<TipoDeImagem, string>>;
  /** Nomes das abas de cada tipo. */
  types?: Partial<Record<TipoDeImagem, string>>;
  /** Rótulo do filtro de fonte. */
  sourceFilter?: string;
  /** Rótulo do filtro de estilo. */
  styleFilter?: string;
  /** Valor padrão dos filtros (ex.: "Todas", "Todos"). */
  allFilter?: string;
  /** Rótulo do botão para selecionar arquivo local. */
  chooseFromComputer?: string;
  /** Rótulo da coluna de metadados: fonte. */
  sourceLabel?: string;
  /** Rótulo da coluna de metadados: tamanho. */
  sizeLabel?: string;
  /** Rótulo da ação primária por tipo (ex.: "Usar esta capa"). */
  useByType?: Partial<Record<TipoDeImagem, string>>;
  /** Rótulo da ação de restaurar a arte original. */
  resetToOriginal?: string;
  /** Rótulo do botão cancelar. */
  cancel?: string;
  /** Mensagem ou texto de carregamento. */
  loading?: string;
  /** Mensagem exibida quando não houver imagens. */
  empty?: string;
  /** Texto exibido quando uma imagem falha ao carregar. */
  noArt?: string;
  /** Etiqueta da imagem atualmente aplicada ao jogo. */
  current?: string;
  /** Formatação do texto de resumo acima da grade. */
  summary?: (total: number, type: TipoDeImagem, source?: string, dimensions?: string) => string;
}

export interface SeletorDeImagensProps {
  /** Controla se o popup está aberto. */
  open: boolean;
  /** Callback acionado ao fechar o diálogo. */
  onClose: () => void;
  /** Callback acionado quando o usuário confirma a escolha de uma imagem. */
  onChoose: (option: OpcaoDeImagem, type: TipoDeImagem) => void;
  /** Título do jogo para exibição no cabeçalho. */
  gameTitle: string;
  /** URL da miniatura da arte de capa do jogo para o cabeçalho. */
  coverThumbnailUrl?: string;
  /** Lista de tipos de imagem disponíveis (padrão: capa, fundo, logo, ícone). */
  types?: readonly TipoDeImagem[];
  /** Tipo de imagem atualmente selecionado (controlado). */
  activeType?: TipoDeImagem;
  /** Callback acionado ao alternar de aba de tipo de imagem. */
  onChangeType?: (type: TipoDeImagem) => void;
  /** Opções de imagem agrupadas por tipo. */
  optionsByType?: Partial<Record<TipoDeImagem, readonly OpcaoDeImagem[]>>;
  /** Indica se as opções de um determinado tipo estão carregando. */
  loadingByType?: Partial<Record<TipoDeImagem, boolean>>;
  /** Indica se a lista de opções de um determinado tipo está vazia. */
  emptyByType?: Partial<Record<TipoDeImagem, boolean>>;
  /** Callback acionado ao clicar em "Escolher do computador…". */
  onChooseFromComputer?: () => void;
  /** Callback acionado ao clicar em "Voltar à original". */
  onResetToOriginal?: () => void;
  /** Catálogo de textos e rótulos para internacionalização. */
  labels?: SeletorDeImagensLabels;
  className?: string;
}

const TIPOS_PADRAO: readonly TipoDeImagem[] = ["capa", "fundo", "logo", "icone"];

const DEFAULT_LABELS = {
  close: "Fechar (Esc)",
  titlePrefix: "Imagens de",
  subtitles: {
    capa: "A capa aparece na grade, na biblioteca e em Voltar a jogar.",
    fundo: "O fundo preenche o destaque e a página do jogo.",
    logo: "O logo aparece sobre o fundo no topo da página do jogo.",
    icone: "O ícone aparece na barra lateral, na lista e nas notificações.",
  },
  types: {
    capa: "Capa",
    fundo: "Fundo",
    logo: "Logo",
    icone: "Ícone",
  },
  sourceFilter: "Fonte:",
  styleFilter: "Estilo:",
  allFilter: "Todas",
  chooseFromComputer: "Escolher do computador…",
  sourceLabel: "Fonte",
  sizeLabel: "Tamanho",
  useByType: {
    capa: "Usar esta capa",
    fundo: "Usar este fundo",
    logo: "Usar este logo",
    icone: "Usar este ícone",
  },
  resetToOriginal: "Voltar à original",
  cancel: "Cancelar",
  loading: "Carregando opções…",
  empty: "Nenhuma imagem encontrada.",
  noArt: "Sem imagem",
  current: "Atual",
} as const;

function calculaProximoIndex(
  key: string,
  index: number,
  total: number,
  colunas: number,
): number | null {
  if (key === "ArrowRight") return (index + 1) % total;
  if (key === "ArrowLeft") return (index - 1 + total) % total;
  if (key === "ArrowDown") return index + colunas < total ? index + colunas : index;
  if (key === "ArrowUp") return index - colunas >= 0 ? index - colunas : index;
  if (key === "Home") return 0;
  if (key === "End") return total - 1;
  return null;
}

/**
 * Diálogo SeletorDeImagens do TAKO (WEB-04g):
 * Permite ao usuário trocar capa, fundo, logo e ícone do jogo com visual fiel ao design system,
 * abas segmentadas por tipo, filtros, grade navegável por teclado, prévia detalhada e estados de carregamento e sem arte.
 */
export function SeletorDeImagens({
  open,
  onClose,
  onChoose,
  gameTitle,
  coverThumbnailUrl,
  types = TIPOS_PADRAO,
  activeType: activeTypeProp,
  onChangeType,
  optionsByType = {},
  loadingByType = {},
  emptyByType = {},
  onChooseFromComputer,
  onResetToOriginal,
  labels: labelsProp,
  className,
}: SeletorDeImagensProps) {
  // Mesclagem de rótulos com os padrões em português
  const labels = useMemo(() => {
    const subtitlesMap: Record<TipoDeImagem, string> = {
      ...DEFAULT_LABELS.subtitles,
      ...labelsProp?.subtitles,
    };
    const typesMap: Record<TipoDeImagem, string> = {
      ...DEFAULT_LABELS.types,
      ...labelsProp?.types,
    };
    const useMap: Record<TipoDeImagem, string> = {
      ...DEFAULT_LABELS.useByType,
      ...labelsProp?.useByType,
    };

    return {
      close: labelsProp?.close ?? DEFAULT_LABELS.close,
      titlePrefix: labelsProp?.titlePrefix ?? DEFAULT_LABELS.titlePrefix,
      subtitles: subtitlesMap,
      types: typesMap,
      sourceFilter: labelsProp?.sourceFilter ?? DEFAULT_LABELS.sourceFilter,
      styleFilter: labelsProp?.styleFilter ?? DEFAULT_LABELS.styleFilter,
      allFilter: labelsProp?.allFilter ?? DEFAULT_LABELS.allFilter,
      chooseFromComputer: labelsProp?.chooseFromComputer ?? DEFAULT_LABELS.chooseFromComputer,
      sourceLabel: labelsProp?.sourceLabel ?? DEFAULT_LABELS.sourceLabel,
      sizeLabel: labelsProp?.sizeLabel ?? DEFAULT_LABELS.sizeLabel,
      useByType: useMap,
      resetToOriginal: labelsProp?.resetToOriginal ?? DEFAULT_LABELS.resetToOriginal,
      cancel: labelsProp?.cancel ?? DEFAULT_LABELS.cancel,
      loading: labelsProp?.loading ?? DEFAULT_LABELS.loading,
      empty: labelsProp?.empty ?? DEFAULT_LABELS.empty,
      noArt: labelsProp?.noArt ?? DEFAULT_LABELS.noArt,
      current: labelsProp?.current ?? DEFAULT_LABELS.current,
      summary: labelsProp?.summary,
    };
  }, [labelsProp]);

  // Controle de tipo ativo (controlado ou interno)
  const [internalActiveType, setInternalActiveType] = useState<TipoDeImagem>(types[0] ?? "capa");
  const activeType = activeTypeProp ?? internalActiveType;

  const handleTrocarTipo = (novoTipo: TipoDeImagem) => {
    if (onChangeType) {
      onChangeType(novoTipo);
    } else {
      setInternalActiveType(novoTipo);
    }
  };

  // Opções para o tipo ativo
  const currentOptions = useMemo(() => {
    return optionsByType[activeType] ?? [];
  }, [optionsByType, activeType]);

  const isLoading = loadingByType[activeType] ?? false;
  const isEmpty = (emptyByType[activeType] ?? false) || (!isLoading && currentOptions.length === 0);

  // Controle de seleção para prévia sem efeitos colaterais síncronos
  const [overrideSelecionadoId, setOverrideSelecionadoId] = useState<string | null>(null);
  const [ultimoTipo, setUltimoTipo] = useState<TipoDeImagem>(activeType);

  if (activeType !== ultimoTipo) {
    setUltimoTipo(activeType);
    setOverrideSelecionadoId(null);
  }

  const idPadrao = useMemo(() => {
    const atual = currentOptions.find((o) => o.current);
    return atual?.id ?? currentOptions[0]?.id ?? null;
  }, [currentOptions]);

  const idSelecionado = overrideSelecionadoId ?? idPadrao;

  const selectedOption = useMemo(() => {
    if (!idSelecionado) return null;
    return currentOptions.find((o) => o.id === idSelecionado) ?? currentOptions[0] ?? null;
  }, [idSelecionado, currentOptions]);

  // Rastreamento de erros de carregamento de imagens individuais (estado sem arte)
  const [falhasDeImagem, setFalhasDeImagem] = useState<Record<string, boolean>>({});

  // Abas para o componente Abas segmentado
  const tabsData: readonly Aba<TipoDeImagem>[] = useMemo(() => {
    return types.map((t) => ({
      id: t,
      label: labels.types[t] ?? t,
    }));
  }, [types, labels.types]);

  // Título e subtítulo do cabeçalho
  const tituloCompleto = `${labels.titlePrefix} ${gameTitle}`;
  const subtituloAtivo = labels.subtitles[activeType] ?? "";

  // Ação primária
  const textoBotaoUsar = labels.useByType[activeType] ?? "Usar esta imagem";

  const handleEscolher = () => {
    if (selectedOption) {
      onChoose(selectedOption, activeType);
    }
  };

  // Grade e número de colunas por tipo
  const numColunas = activeType === "fundo" || activeType === "logo" ? 3 : 6;
  const classeGrade =
    activeType === "fundo"
      ? styles["gradeFundo"]
      : activeType === "logo"
        ? styles["gradeLogo"]
        : activeType === "icone"
          ? styles["gradeIcone"]
          : styles["gradeCapa"];

  const classeAspect =
    activeType === "fundo"
      ? styles["aspectFundo"]
      : activeType === "logo"
        ? styles["aspectLogo"]
        : activeType === "icone"
          ? styles["aspectIcone"]
          : styles["aspectCapa"];

  // Navegação por teclado na grade
  const gridRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const handleKeyDownGrade = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const total = currentOptions.length;
    if (total === 0) return;

    if (event.key === "Enter") {
      event.preventDefault();
      const opcao = currentOptions[index];
      if (opcao) {
        onChoose(opcao, activeType);
      }
      return;
    }

    const proximo = calculaProximoIndex(event.key, index, total, numColunas);
    if (proximo !== null) {
      event.preventDefault();
      if (proximo !== index) {
        const proximoItem = currentOptions[proximo];
        if (proximoItem) {
          setOverrideSelecionadoId(proximoItem.id);
          gridRefs.current[proximo]?.focus();
        }
      }
    }
  };

  // Texto de resumo informativo
  const textoResumo = useMemo(() => {
    if (labels.summary) {
      return labels.summary(
        currentOptions.length,
        activeType,
        selectedOption?.source,
        selectedOption?.dimensions,
      );
    }
    const nomeTipoPlural =
      activeType === "capa"
        ? "capas"
        : activeType === "fundo"
          ? "fundos"
          : activeType === "logo"
            ? "logos"
            : "ícones";
    const partes: string[] = [`${currentOptions.length} ${nomeTipoPlural}`];
    if (selectedOption?.source) partes.push(selectedOption.source);
    if (selectedOption?.dimensions) partes.push(selectedOption.dimensions);
    return partes.join(" · ");
  }, [labels, currentOptions.length, activeType, selectedOption]);

  return (
    <Dialogo
      open={open}
      onClose={onClose}
      size="l"
      title={tituloCompleto}
      closeLabel={labels.close}
      className={cx(styles["dialogo"], className)}
    >
      {/* Cabeçalho com miniatura da capa do jogo, títulos e botão fechar */}
      <div className={styles["cabecalho"]}>
        {coverThumbnailUrl ? (
          <img
            src={coverThumbnailUrl}
            alt=""
            className={styles["miniatura"]}
            onError={(e) => {
              e.currentTarget.style.display = "none";
            }}
          />
        ) : (
          <span className={styles["miniaturaVazia"]}>
            <Icon name="image" size="md" />
          </span>
        )}
        <div className={styles["titulos"]}>
          <h3 className={styles["titulo"]}>{tituloCompleto}</h3>
          {subtituloAtivo && <p className={styles["subtitulo"]}>{subtituloAtivo}</p>}
        </div>
        <button
          type="button"
          className={styles["botaoFechar"]}
          aria-label={labels.close}
          data-autofocus-last=""
          onClick={onClose}
        >
          <Icon name="x" />
        </button>
      </div>

      {/* Barra de controles: abas de tipo, seletores e escolher do computador */}
      <div className={styles["barraControles"]}>
        <Abas
          variant="segmentado"
          tabs={tabsData}
          value={activeType}
          onChange={(novaAba) => {
            handleTrocarTipo(novaAba);
          }}
          label="Tipos de imagem"
        />

        <div className={styles["filtrosWrap"]}>
          <button type="button" className={styles["seletorFiltro"]}>
            <span>{labels.sourceFilter} {labels.allFilter}</span>
            <Icon name="updown" size="sm" />
          </button>
          <button type="button" className={styles["seletorFiltro"]}>
            <span>{labels.styleFilter} {labels.allFilter}</span>
            <Icon name="updown" size="sm" />
          </button>
        </div>

        <div className={styles["acoesBarra"]}>
          <BotaoDesktop
            variant="outlined"
            icon="folder"
            onClick={onChooseFromComputer}
          >
            {labels.chooseFromComputer}
          </BotaoDesktop>
        </div>
      </div>

      {/* Área principal: Grade à esquerda e Cartão de prévia à direita */}
      <div className={styles["areaPrincipal"]}>
        <div className={styles["colunaEsquerda"]}>
          {!isLoading && !isEmpty && (
            <div className={styles["resumoInfo"]}>{textoResumo}</div>
          )}

          {isLoading && (
            <div
              className={cx(styles["gradeOpcoes"], classeGrade)}
              aria-busy="true"
              aria-label={labels.loading}
            >
              {Array.from({ length: numColunas * 2 }).map((_, idx) => (
                <div
                  key={`skeleton-${idx}`}
                  className={cx(styles["skeletonCard"], classeAspect)}
                />
              ))}
            </div>
          )}

          {!isLoading && isEmpty && (
            <div className={styles["estadoVazio"]} role="status">
              <Icon name="image" size="md" />
              <p>{labels.empty}</p>
            </div>
          )}

          {!isLoading && !isEmpty && (
            <div
              className={cx(styles["gradeOpcoes"], classeGrade)}
              role="grid"
              aria-label={labels.types[activeType] ?? activeType}
            >
              {currentOptions.map((opcao, idx) => {
                const estaSelecionada = opcao.id === idSelecionado;
                const temFalha = falhasDeImagem[opcao.id] || !opcao.url;

                return (
                  <button
                    key={opcao.id}
                    ref={(node) => {
                      gridRefs.current[idx] = node;
                    }}
                    type="button"
                    role="gridcell"
                    aria-selected={estaSelecionada}
                    aria-label={`${labels.types[activeType] ?? "Opção"} ${idx + 1}${opcao.current ? ` (${labels.current})` : ""}`}
                    className={cx(
                      styles["opcaoItem"],
                      classeAspect,
                      estaSelecionada && styles["opcaoSelecionada"],
                    )}
                    onClick={() => {
                      setOverrideSelecionadoId(opcao.id);
                    }}
                    onDoubleClick={() => {
                      onChoose(opcao, activeType);
                    }}
                    onKeyDown={(e) => {
                      handleKeyDownGrade(e, idx);
                    }}
                  >
                    {temFalha ? (
                      <div className={styles["semArte"]}>
                        <Icon name="image" size="sm" />
                        <span className={styles["semArteTexto"]}>
                          {labels.noArt}
                        </span>
                      </div>
                    ) : (
                      <img
                        src={opcao.url}
                        alt=""
                        loading="lazy"
                        className={cx(
                          styles["opcaoImagem"],
                          activeType === "logo" && styles["opcaoImagemLogo"],
                        )}
                        onError={() => {
                          setFalhasDeImagem((prev) => ({
                            ...prev,
                            [opcao.id]: true,
                          }));
                        }}
                      />
                    )}

                    {opcao.current && (
                      <span className={styles["etiquetaAtual"]}>
                        {labels.current}
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          )}
        </div>

        {/* Coluna direita: cartão de prévia e botões de ação */}
        <div className={styles["colunaPrevia"]}>
          <div className={cx(styles["previaImagemWrap"], classeAspect)}>
            {selectedOption && !falhasDeImagem[selectedOption.id] && selectedOption.url ? (
              <img
                src={selectedOption.url}
                alt=""
                className={cx(
                  styles["imagemPrevia"],
                  classeAspect,
                  activeType === "logo" && styles["imagemPreviaLogo"],
                )}
                onError={() => {
                  if (selectedOption) {
                    setFalhasDeImagem((prev) => ({
                      ...prev,
                      [selectedOption.id]: true,
                    }));
                  }
                }}
              />
            ) : (
              <div className={styles["semArte"]}>
                <Icon name="image" size="md" />
                <span className={styles["semArteTexto"]}>
                  {labels.noArt}
                </span>
              </div>
            )}
          </div>

          <div className={styles["linhaMeta"]}>
            <span>{labels.sourceLabel}</span>
            <strong className={styles["linhaMetaValor"]}>
              {selectedOption?.source ?? "—"}
            </strong>
          </div>

          <div className={styles["linhaMeta"]}>
            <span>{labels.sizeLabel}</span>
            <strong className={styles["linhaMetaValor"]}>
              {selectedOption?.dimensions ?? "—"}
            </strong>
          </div>

          <div className={styles["acoesPrevia"]}>
            <BotaoDesktop
              variant="primary"
              full
              icon="check"
              disabled={!selectedOption}
              onClick={handleEscolher}
            >
              {textoBotaoUsar}
            </BotaoDesktop>

            <div className={styles["acoesSecundarias"]}>
              <BotaoDesktop
                variant="outlined"
                disabled={!onResetToOriginal}
                onClick={onResetToOriginal}
              >
                {labels.resetToOriginal}
              </BotaoDesktop>
              <BotaoDesktop variant="outlined" onClick={onClose}>
                {labels.cancel}
              </BotaoDesktop>
            </div>
          </div>
        </div>
      </div>
    </Dialogo>
  );
}
