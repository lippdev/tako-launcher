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
  fonte: string;
  /** Dimensões da arte (ex.: "600 × 900", "1920 × 1080"). */
  dimensoes?: string;
  /** Estilo da arte para filtros (ex.: "Oficial", "Alternativa"). */
  estilo?: string;
  /** Indica se esta imagem é a que está atualmente definida para o jogo. */
  atual?: boolean;
}

export interface SeletorDeImagensRotulos {
  /** Rótulo do botão fechar (acessibilidade / aria-label). */
  fechar?: string;
  /** Prefixo do título do diálogo (ex.: "Imagens de"). */
  tituloPrefixo?: string;
  /** Subtítulo descritivo por tipo de imagem. */
  subtitulos?: Partial<Record<TipoDeImagem, string>>;
  /** Nomes das abas de cada tipo. */
  tipos?: Partial<Record<TipoDeImagem, string>>;
  /** Rótulo do filtro de fonte. */
  filtroFonte?: string;
  /** Rótulo do filtro de estilo. */
  filtroEstilo?: string;
  /** Valor padrão dos filtros (ex.: "Todas", "Todos"). */
  filtroTodas?: string;
  /** Rótulo do botão para selecionar arquivo local. */
  escolherDoComputador?: string;
  /** Rótulo da coluna de metadados: fonte. */
  rotuloFonte?: string;
  /** Rótulo da coluna de metadados: tamanho. */
  rotuloTamanho?: string;
  /** Rótulo da ação primária por tipo (ex.: "Usar esta capa"). */
  usarPorTipo?: Partial<Record<TipoDeImagem, string>>;
  /** Rótulo da ação de restaurar a arte original. */
  voltarAoOriginal?: string;
  /** Rótulo do botão cancelar. */
  cancelar?: string;
  /** Mensagem ou texto de carregamento. */
  carregando?: string;
  /** Mensagem exibida quando não houver imagens. */
  vazio?: string;
  /** Texto exibido quando uma imagem falha ao carregar. */
  semArte?: string;
  /** Etiqueta da imagem atualmente aplicada ao jogo. */
  atual?: string;
  /** Formatação do texto de resumo acima da grade. */
  resumo?: (total: number, tipo: TipoDeImagem, fonte?: string, dimensoes?: string) => string;
}

export interface SeletorDeImagensProps {
  /** Controla se o popup está aberto. */
  open: boolean;
  /** Callback acionado ao fechar o diálogo. */
  onFechar: () => void;
  /** Callback alternativo de fechamento (alias de `onFechar`). */
  onClose?: () => void;
  /** Callback acionado quando o usuário confirma a escolha de uma imagem. */
  onEscolher: (opcao: OpcaoDeImagem, tipo: TipoDeImagem) => void;
  /** Título do jogo para exibição no cabeçalho. */
  tituloDoJogo: string;
  /** URL da miniatura da arte de capa do jogo para o cabeçalho. */
  capaMiniaturaUrl?: string;
  /** Lista de tipos de imagem disponíveis (padrão: capa, fundo, logo, ícone). */
  tipos?: readonly TipoDeImagem[];
  /** Tipo de imagem atualmente selecionado (controlado). */
  tipoAtivo?: TipoDeImagem;
  /** Callback acionado ao alternar de aba de tipo de imagem. */
  onTrocarTipo?: (tipo: TipoDeImagem) => void;
  /** Opções de imagem agrupadas por tipo. */
  opcoesPorTipo?: Partial<Record<TipoDeImagem, readonly OpcaoDeImagem[]>>;
  /** Indica se as opções de um determinado tipo estão carregando. */
  carregandoPorTipo?: Partial<Record<TipoDeImagem, boolean>>;
  /** Indica se a lista de opções de um determinado tipo está vazia. */
  vazioPorTipo?: Partial<Record<TipoDeImagem, boolean>>;
  /** Callback acionado ao clicar em "Escolher do computador…". */
  onEscolherDoComputador?: () => void;
  /** Callback acionado ao clicar em "Voltar à original". */
  onVoltarAoOriginal?: () => void;
  /** Catálogo de textos e rótulos para internacionalização. */
  rotulos?: SeletorDeImagensRotulos;
  className?: string;
}

const TIPOS_PADRAO: readonly TipoDeImagem[] = ["capa", "fundo", "logo", "icone"];

const ROTULOS_PADRAO = {
  fechar: "Fechar (Esc)",
  tituloPrefixo: "Imagens de",
  subtitulos: {
    capa: "A capa aparece na grade, na biblioteca e em Voltar a jogar.",
    fundo: "O fundo preenche o destaque e a página do jogo.",
    logo: "O logo aparece sobre o fundo no topo da página do jogo.",
    icone: "O ícone aparece na barra lateral, na lista e nas notificações.",
  },
  tipos: {
    capa: "Capa",
    fundo: "Fundo",
    logo: "Logo",
    icone: "Ícone",
  },
  filtroFonte: "Fonte:",
  filtroEstilo: "Estilo:",
  filtroTodas: "Todas",
  escolherDoComputador: "Escolher do computador…",
  rotuloFonte: "Fonte",
  rotuloTamanho: "Tamanho",
  usarPorTipo: {
    capa: "Usar esta capa",
    fundo: "Usar este fundo",
    logo: "Usar este logo",
    icone: "Usar este ícone",
  },
  voltarAoOriginal: "Voltar à original",
  cancelar: "Cancelar",
  carregando: "Carregando opções…",
  vazio: "Nenhuma imagem encontrada.",
  semArte: "Sem imagem",
  atual: "Atual",
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
  onFechar,
  onClose,
  onEscolher,
  tituloDoJogo,
  capaMiniaturaUrl,
  tipos = TIPOS_PADRAO,
  tipoAtivo: tipoAtivoProp,
  onTrocarTipo,
  opcoesPorTipo = {},
  carregandoPorTipo = {},
  vazioPorTipo = {},
  onEscolherDoComputador,
  onVoltarAoOriginal,
  rotulos: rotulosProp,
  className,
}: SeletorDeImagensProps) {
  const fechar = onClose ?? onFechar;

  // Mesclagem de rótulos com os padrões em português
  const rotulos = useMemo(() => {
    const subtitulosMap: Record<TipoDeImagem, string> = {
      ...ROTULOS_PADRAO.subtitulos,
      ...rotulosProp?.subtitulos,
    };
    const tiposMap: Record<TipoDeImagem, string> = {
      ...ROTULOS_PADRAO.tipos,
      ...rotulosProp?.tipos,
    };
    const usarMap: Record<TipoDeImagem, string> = {
      ...ROTULOS_PADRAO.usarPorTipo,
      ...rotulosProp?.usarPorTipo,
    };

    return {
      fechar: rotulosProp?.fechar ?? ROTULOS_PADRAO.fechar,
      tituloPrefixo: rotulosProp?.tituloPrefixo ?? ROTULOS_PADRAO.tituloPrefixo,
      subtitulos: subtitulosMap,
      tipos: tiposMap,
      filtroFonte: rotulosProp?.filtroFonte ?? ROTULOS_PADRAO.filtroFonte,
      filtroEstilo: rotulosProp?.filtroEstilo ?? ROTULOS_PADRAO.filtroEstilo,
      filtroTodas: rotulosProp?.filtroTodas ?? ROTULOS_PADRAO.filtroTodas,
      escolherDoComputador: rotulosProp?.escolherDoComputador ?? ROTULOS_PADRAO.escolherDoComputador,
      rotuloFonte: rotulosProp?.rotuloFonte ?? ROTULOS_PADRAO.rotuloFonte,
      rotuloTamanho: rotulosProp?.rotuloTamanho ?? ROTULOS_PADRAO.rotuloTamanho,
      usarPorTipo: usarMap,
      voltarAoOriginal: rotulosProp?.voltarAoOriginal ?? ROTULOS_PADRAO.voltarAoOriginal,
      cancelar: rotulosProp?.cancelar ?? ROTULOS_PADRAO.cancelar,
      carregando: rotulosProp?.carregando ?? ROTULOS_PADRAO.carregando,
      vazio: rotulosProp?.vazio ?? ROTULOS_PADRAO.vazio,
      semArte: rotulosProp?.semArte ?? ROTULOS_PADRAO.semArte,
      atual: rotulosProp?.atual ?? ROTULOS_PADRAO.atual,
      resumo: rotulosProp?.resumo,
    };
  }, [rotulosProp]);

  // Controle de tipo ativo (controlado ou interno)
  const [tipoInterno, setTipoInterno] = useState<TipoDeImagem>(tipos[0] ?? "capa");
  const tipoAtivo = tipoAtivoProp ?? tipoInterno;

  const handleTrocarTipo = (novoTipo: TipoDeImagem) => {
    if (onTrocarTipo) {
      onTrocarTipo(novoTipo);
    } else {
      setTipoInterno(novoTipo);
    }
  };

  // Opções para o tipo ativo
  const opcoesAtuais = useMemo(() => {
    return opcoesPorTipo[tipoAtivo] ?? [];
  }, [opcoesPorTipo, tipoAtivo]);

  const estaCarregando = carregandoPorTipo[tipoAtivo] ?? false;
  const estaVazio = (vazioPorTipo[tipoAtivo] ?? false) || (!estaCarregando && opcoesAtuais.length === 0);

  // Controle de seleção para prévia sem efeitos colaterais síncronos
  const [overrideSelecionadoId, setOverrideSelecionadoId] = useState<string | null>(null);
  const [ultimoTipo, setUltimoTipo] = useState<TipoDeImagem>(tipoAtivo);

  if (tipoAtivo !== ultimoTipo) {
    setUltimoTipo(tipoAtivo);
    setOverrideSelecionadoId(null);
  }

  const idPadrao = useMemo(() => {
    const atual = opcoesAtuais.find((o) => o.atual);
    return atual?.id ?? opcoesAtuais[0]?.id ?? null;
  }, [opcoesAtuais]);

  const idSelecionado = overrideSelecionadoId ?? idPadrao;
  const opcaoSelecionada = useMemo(() => {
    if (!idSelecionado) return null;
    return opcoesAtuais.find((o) => o.id === idSelecionado) ?? opcoesAtuais[0] ?? null;
  }, [idSelecionado, opcoesAtuais]);

  // Rastreamento de erros de carregamento de imagens individuais (estado sem arte)
  const [falhasDeImagem, setFalhasDeImagem] = useState<Record<string, boolean>>({});

  // Abas para o componente Abas segmentado
  const tabsData: readonly Aba<TipoDeImagem>[] = useMemo(() => {
    return tipos.map((t) => ({
      id: t,
      label: rotulos.tipos[t] ?? t,
    }));
  }, [tipos, rotulos.tipos]);

  // Título e subtítulo do cabeçalho
  const tituloCompleto = `${rotulos.tituloPrefixo} ${tituloDoJogo}`;
  const subtituloAtivo = rotulos.subtitulos[tipoAtivo] ?? "";

  // Ação primária
  const textoBotaoUsar = rotulos.usarPorTipo[tipoAtivo] ?? "Usar esta imagem";

  const handleEscolher = () => {
    if (opcaoSelecionada) {
      onEscolher(opcaoSelecionada, tipoAtivo);
    }
  };

  // Grade e número de colunas por tipo
  const numColunas = tipoAtivo === "fundo" || tipoAtivo === "logo" ? 3 : 6;
  const classeGrade =
    tipoAtivo === "fundo"
      ? styles["gradeFundo"]
      : tipoAtivo === "logo"
        ? styles["gradeLogo"]
        : tipoAtivo === "icone"
          ? styles["gradeIcone"]
          : styles["gradeCapa"];

  const classeAspect =
    tipoAtivo === "fundo"
      ? styles["aspectFundo"]
      : tipoAtivo === "logo"
        ? styles["aspectLogo"]
        : tipoAtivo === "icone"
          ? styles["aspectIcone"]
          : styles["aspectCapa"];

  // Navegação por teclado na grade
  const gridRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const handleKeyDownGrade = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const total = opcoesAtuais.length;
    if (total === 0) return;

    if (event.key === "Enter") {
      event.preventDefault();
      const opcao = opcoesAtuais[index];
      if (opcao) {
        onEscolher(opcao, tipoAtivo);
      }
      return;
    }

    const proximo = calculaProximoIndex(event.key, index, total, numColunas);
    if (proximo !== null) {
      event.preventDefault();
      if (proximo !== index) {
        const proximoItem = opcoesAtuais[proximo];
        if (proximoItem) {
          setOverrideSelecionadoId(proximoItem.id);
          gridRefs.current[proximo]?.focus();
        }
      }
    }
  };

  // Texto de resumo informativo
  const textoResumo = useMemo(() => {
    if (rotulos.resumo) {
      return rotulos.resumo(
        opcoesAtuais.length,
        tipoAtivo,
        opcaoSelecionada?.fonte,
        opcaoSelecionada?.dimensoes,
      );
    }
    const nomeTipoPlural =
      tipoAtivo === "capa"
        ? "capas"
        : tipoAtivo === "fundo"
          ? "fundos"
          : tipoAtivo === "logo"
            ? "logos"
            : "ícones";
    const partes: string[] = [`${opcoesAtuais.length} ${nomeTipoPlural}`];
    if (opcaoSelecionada?.fonte) partes.push(opcaoSelecionada.fonte);
    if (opcaoSelecionada?.dimensoes) partes.push(opcaoSelecionada.dimensoes);
    return partes.join(" · ");
  }, [rotulos, opcoesAtuais.length, tipoAtivo, opcaoSelecionada]);

  return (
    <Dialogo
      open={open}
      onClose={fechar}
      size="l"
      title={tituloCompleto}
      closeLabel={rotulos.fechar}
      className={cx(styles["dialogo"], className)}
    >
      {/* Cabeçalho com miniatura da capa do jogo, títulos e botão fechar */}
      <div className={styles["cabecalho"]}>
        {capaMiniaturaUrl ? (
          <img
            src={capaMiniaturaUrl}
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
          aria-label={rotulos.fechar}
          data-autofocus-last=""
          onClick={fechar}
        >
          <Icon name="x" />
        </button>
      </div>

      {/* Barra de controles: abas de tipo, seletores e escolher do computador */}
      <div className={styles["barraControles"]}>
        <Abas
          variant="segmentado"
          tabs={tabsData}
          value={tipoAtivo}
          onChange={(novaAba) => {
            handleTrocarTipo(novaAba);
          }}
          label="Tipos de imagem"
        />

        <div className={styles["filtrosWrap"]}>
          <button type="button" className={styles["seletorFiltro"]}>
            <span>{rotulos.filtroFonte} {rotulos.filtroTodas}</span>
            <Icon name="updown" size="sm" />
          </button>
          <button type="button" className={styles["seletorFiltro"]}>
            <span>{rotulos.filtroEstilo} {rotulos.filtroTodas}</span>
            <Icon name="updown" size="sm" />
          </button>
        </div>

        <div className={styles["acoesBarra"]}>
          <BotaoDesktop
            variant="outlined"
            icon="folder"
            onClick={onEscolherDoComputador}
          >
            {rotulos.escolherDoComputador}
          </BotaoDesktop>
        </div>
      </div>

      {/* Área principal: Grade à esquerda e Cartão de prévia à direita */}
      <div className={styles["areaPrincipal"]}>
        <div className={styles["colunaEsquerda"]}>
          {!estaCarregando && !estaVazio && (
            <div className={styles["resumoInfo"]}>{textoResumo}</div>
          )}

          {estaCarregando && (
            <div
              className={cx(styles["gradeOpcoes"], classeGrade)}
              aria-busy="true"
              aria-label={rotulos.carregando}
            >
              {Array.from({ length: numColunas * 2 }).map((_, idx) => (
                <div
                  key={`skeleton-${idx}`}
                  className={cx(styles["skeletonCard"], classeAspect)}
                />
              ))}
            </div>
          )}

          {!estaCarregando && estaVazio && (
            <div className={styles["estadoVazio"]} role="status">
              <Icon name="image" size="md" />
              <p>{rotulos.vazio}</p>
            </div>
          )}

          {!estaCarregando && !estaVazio && (
            <div
              className={cx(styles["gradeOpcoes"], classeGrade)}
              role="grid"
              aria-label={rotulos.tipos[tipoAtivo] ?? tipoAtivo}
            >
              {opcoesAtuais.map((opcao, idx) => {
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
                    aria-label={`${rotulos.tipos[tipoAtivo] ?? "Opção"} ${idx + 1}${opcao.atual ? ` (${rotulos.atual})` : ""}`}
                    className={cx(
                      styles["opcaoItem"],
                      classeAspect,
                      estaSelecionada && styles["opcaoSelecionada"],
                    )}
                    onClick={() => {
                      setOverrideSelecionadoId(opcao.id);
                    }}
                    onDoubleClick={() => {
                      onEscolher(opcao, tipoAtivo);
                    }}
                    onKeyDown={(e) => {
                      handleKeyDownGrade(e, idx);
                    }}
                  >
                    {temFalha ? (
                      <div className={styles["semArte"]}>
                        <Icon name="image" size="sm" />
                        <span className={styles["semArteTexto"]}>
                          {rotulos.semArte}
                        </span>
                      </div>
                    ) : (
                      <img
                        src={opcao.url}
                        alt=""
                        loading="lazy"
                        className={cx(
                          styles["opcaoImagem"],
                          tipoAtivo === "logo" && styles["opcaoImagemLogo"],
                        )}
                        onError={() => {
                          setFalhasDeImagem((prev) => ({
                            ...prev,
                            [opcao.id]: true,
                          }));
                        }}
                      />
                    )}

                    {opcao.atual && (
                      <span className={styles["etiquetaAtual"]}>
                        {rotulos.atual}
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
            {opcaoSelecionada && !falhasDeImagem[opcaoSelecionada.id] && opcaoSelecionada.url ? (
              <img
                src={opcaoSelecionada.url}
                alt=""
                className={cx(
                  styles["imagemPrevia"],
                  classeAspect,
                  tipoAtivo === "logo" && styles["imagemPreviaLogo"],
                )}
                onError={() => {
                  if (opcaoSelecionada) {
                    setFalhasDeImagem((prev) => ({
                      ...prev,
                      [opcaoSelecionada.id]: true,
                    }));
                  }
                }}
              />
            ) : (
              <div className={styles["semArte"]}>
                <Icon name="image" size="md" />
                <span className={styles["semArteTexto"]}>
                  {rotulos.semArte}
                </span>
              </div>
            )}
          </div>

          <div className={styles["linhaMeta"]}>
            <span>{rotulos.rotuloFonte}</span>
            <strong className={styles["linhaMetaValor"]}>
              {opcaoSelecionada?.fonte ?? "—"}
            </strong>
          </div>

          <div className={styles["linhaMeta"]}>
            <span>{rotulos.rotuloTamanho}</span>
            <strong className={styles["linhaMetaValor"]}>
              {opcaoSelecionada?.dimensoes ?? "—"}
            </strong>
          </div>

          <div className={styles["acoesPrevia"]}>
            <BotaoDesktop
              variant="primary"
              full
              icon="check"
              disabled={!opcaoSelecionada}
              onClick={handleEscolher}
            >
              {textoBotaoUsar}
            </BotaoDesktop>

            <div className={styles["acoesSecundarias"]}>
              <BotaoDesktop
                variant="outlined"
                disabled={!onVoltarAoOriginal}
                onClick={onVoltarAoOriginal}
              >
                {rotulos.voltarAoOriginal}
              </BotaoDesktop>
              <BotaoDesktop variant="outlined" onClick={fechar}>
                {rotulos.cancelar}
              </BotaoDesktop>
            </div>
          </div>
        </div>
      </div>
    </Dialogo>
  );
}
