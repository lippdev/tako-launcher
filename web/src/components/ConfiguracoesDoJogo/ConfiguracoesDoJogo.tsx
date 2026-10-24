import { useMemo, type ReactNode } from "react";
import { cx } from "../../lib/cx";
import { PainelLateral } from "../PainelLateral/PainelLateral";
import { GrupoDeConfiguracao, LinhaDeConfiguracao, type LinhaIds } from "../LinhaDeConfiguracao/LinhaDeConfiguracao";
import { Chave } from "../Chave/Chave";
import { CampoDeSelecao, type OpcaoDeSelecao } from "../CampoDeSelecao/CampoDeSelecao";
import { Icon } from "../Icon/Icon";
import styles from "./ConfiguracoesDoJogo.module.css";

export type { OpcaoDeSelecao };

export type ChaveConfiguracaoJogo =
  | "monitor"
  | "resolucaoETaxa"
  | "hdr"
  | "vrr"
  | "saidaDeAudio"
  | "argumentos"
  | "pastaDeTrabalho"
  | "variaveisDeAmbiente"
  | "comandosAntesEDepois";

export type ValorConfiguracaoJogo = boolean | string;

export interface ValoresConfiguracoesDoJogo {
  monitor?: boolean | string;
  resolucaoETaxa?: string;
  hdr?: string;
  vrr?: string | boolean;
  saidaDeAudio?: string;
  argumentos?: string;
  pastaDeTrabalho?: string;
  variaveisDeAmbiente?: string;
  comandosAntesEDepois?: string;
}

export interface OpcoesConfiguracoesDoJogo {
  monitor?: readonly OpcaoDeSelecao<string>[];
  resolucaoETaxa?: readonly OpcaoDeSelecao<string>[];
  hdr?: readonly OpcaoDeSelecao<string>[];
  vrr?: readonly OpcaoDeSelecao<string>[];
  saidaDeAudio?: readonly OpcaoDeSelecao<string>[];
}

export interface DescricoesConfiguracoesDoJogo {
  monitor?: string;
  resolucaoETaxa?: string;
  hdr?: string;
  vrr?: string;
  saidaDeAudio?: string;
  argumentos?: string;
  pastaDeTrabalho?: string;
  variaveisDeAmbiente?: string;
  comandosAntesEDepois?: string;
}

export interface ConfiguracoesDoJogoRotulos {
  titulo: string;
  fechar: string;
  grupoTela: string;
  grupoSom: string;
  grupoAbrirOJogo: string;
  monitor: string;
  resolucaoETaxa: string;
  hdr: string;
  vrr: string;
  saidaDeAudio: string;
  argumentos: string;
  pastaDeTrabalho: string;
  variaveisDeAmbiente: string;
  comandosAntesEDepois: string;
  usarTudoDoTako: string;
  descricaoUsarTudoDoTako?: string | ((mudancas: number) => string);
  padraoPrefixo?: string;
  nenhum?: string;
  nenhuma?: string;
  ligado?: string;
  desligado?: string;
  singularMudanca?: string;
  pluralMudancas?: string;
  nenhumaMudanca?: string;
}

export interface ConfiguracoesDoJogoProps {
  open?: boolean;
  onClose?: () => void;
  tituloDoJogo: string;
  capaUrl?: string;
  valores: ValoresConfiguracoesDoJogo;
  padroes?: Partial<ValoresConfiguracoesDoJogo>;
  opcoes?: OpcoesConfiguracoesDoJogo;
  descricoes?: DescricoesConfiguracoesDoJogo;
  rotulos: ConfiguracoesDoJogoRotulos;
  mudancasCount?: number;
  inline?: boolean;
  onChange?: (chave: ChaveConfiguracaoJogo, valor: ValorConfiguracaoJogo) => void;
  onResetarAoPadrao?: () => void;
  onCliqueLinha?: (chave: ChaveConfiguracaoJogo) => void;
  className?: string;
}

const CHAVES_TELA: readonly ChaveConfiguracaoJogo[] = ["monitor", "resolucaoETaxa", "hdr", "vrr"];
const CHAVES_SOM: readonly ChaveConfiguracaoJogo[] = ["saidaDeAudio"];
const CHAVES_ABRIR: readonly ChaveConfiguracaoJogo[] = [
  "argumentos",
  "pastaDeTrabalho",
  "variaveisDeAmbiente",
  "comandosAntesEDepois",
];

interface LinhaPropsBase {
  title: string;
  description?: string;
  tag?: string;
  danger?: boolean;
  control?: ReactNode | ((ids: LinhaIds) => ReactNode);
  onClick?: () => void;
  value?: string;
}

export function ConfiguracoesDoJogo({
  open = true,
  onClose,
  tituloDoJogo,
  capaUrl,
  valores,
  padroes,
  opcoes,
  descricoes,
  rotulos,
  mudancasCount: mudancasCountProp,
  inline = false,
  onChange,
  onResetarAoPadrao,
  onCliqueLinha,
  className,
}: ConfiguracoesDoJogoProps) {
  const padraoPrefixo = rotulos.padraoPrefixo ?? "Padrao: ";

  const formatarRotuloValorPadrao = (chave: ChaveConfiguracaoJogo, valorPadrao: ValorConfiguracaoJogo): string => {
    if (typeof valorPadrao === "boolean") {
      return valorPadrao ? (rotulos.ligado ?? "ligado") : (rotulos.desligado ?? "desligado");
    }
    const listaOpcoes = opcoes?.[chave as keyof OpcoesConfiguracoesDoJogo];
    if (listaOpcoes) {
      const encontrada = listaOpcoes.find((o: OpcaoDeSelecao<string>) => o.value === String(valorPadrao));
      if (encontrada) {
        return encontrada.label;
      }
    }
    if (typeof valorPadrao === "string" && valorPadrao.length > 0) {
      return valorPadrao;
    }
    if (chave === "variaveisDeAmbiente") {
      return rotulos.nenhuma ?? "nenhuma";
    }
    return rotulos.nenhum ?? "nenhum";
  };

  const chavesMudadas = useMemo(() => {
    if (!padroes) return [];
    const chaves: ChaveConfiguracaoJogo[] = [];
    const todas: ChaveConfiguracaoJogo[] = [...CHAVES_TELA, ...CHAVES_SOM, ...CHAVES_ABRIR];
    for (const ch of todas) {
      const val = valores[ch];
      const def = padroes[ch];
      if (val !== undefined && def !== undefined && val !== def) {
        chaves.push(ch);
      }
    }
    return chaves;
  }, [valores, padroes]);

  const mudancas = mudancasCountProp !== undefined ? mudancasCountProp : chavesMudadas.length;

  const singularMudanca = rotulos.singularMudanca ?? "mudanca";
  const pluralMudancas = rotulos.pluralMudancas ?? "mudancas";
  const nenhumaMudanca = rotulos.nenhumaMudanca ?? "nenhuma mudanca";

  const textoMudancas =
    mudancas === 0
      ? nenhumaMudanca
      : mudancas === 1
        ? `1 ${singularMudanca}`
        : `${mudancas} ${pluralMudancas}`;

  const subtituloPainel = `${tituloDoJogo} \u00b7 ${textoMudancas}`;

  const renderLinha = (chave: ChaveConfiguracaoJogo): ReactNode => {
    const valor = valores[chave];
    if (valor === undefined || valor === null) {
      return null;
    }

    const valorPadrao = padroes?.[chave];
    const mudou = valorPadrao !== undefined && valor !== valorPadrao;

    let descricaoFinal: string | undefined = descricoes?.[chave];
    if (mudou && valorPadrao !== undefined) {
      descricaoFinal = `${padraoPrefixo}${formatarRotuloValorPadrao(chave, valorPadrao)}`;
    } else if (!descricaoFinal) {
      if (chave === "argumentos" || chave === "comandosAntesEDepois") {
        descricaoFinal = (typeof valor === "string" && valor.trim()) ? undefined : (rotulos.nenhum ?? "Nenhum");
      } else if (chave === "variaveisDeAmbiente") {
        descricaoFinal = (typeof valor === "string" && valor.trim()) ? undefined : (rotulos.nenhuma ?? "Nenhuma");
      }
    }

    const tituloLinha = rotulos[chave as keyof ConfiguracoesDoJogoRotulos] as string;
    const linhaProps: LinhaPropsBase = { title: tituloLinha };
    if (descricaoFinal) {
      linhaProps.description = descricaoFinal;
    }

    if (chave === "monitor") {
      const opcoesMonitor = opcoes?.monitor;
      if (opcoesMonitor && opcoesMonitor.length > 0) {
        linhaProps.control = ({ labelId, descriptionId }: LinhaIds) => (
          <CampoDeSelecao
            label={tituloLinha}
            options={opcoesMonitor}
            value={String(valor)}
            onChange={(novo) => onChange?.("monitor", novo)}
            aria-labelledby={labelId}
            aria-describedby={descriptionId}
          />
        );
      } else {
        linhaProps.control = ({ labelId, descriptionId }: LinhaIds) => (
          <Chave
            checked={Boolean(valor)}
            onChange={(novo) => onChange?.("monitor", novo)}
            aria-labelledby={labelId}
            aria-describedby={descriptionId}
          />
        );
      }
      return <LinhaDeConfiguracao key={chave} {...linhaProps} />;
    }

    if (chave === "resolucaoETaxa" || chave === "hdr" || chave === "saidaDeAudio") {
      const listaOpcoes = opcoes?.[chave] ?? [];
      linhaProps.control = ({ labelId, descriptionId }: LinhaIds) => (
        <CampoDeSelecao
          label={tituloLinha}
          options={listaOpcoes}
          value={String(valor)}
          onChange={(novo) => onChange?.(chave, novo)}
          aria-labelledby={labelId}
          aria-describedby={descriptionId}
        />
      );
      return <LinhaDeConfiguracao key={chave} {...linhaProps} />;
    }

    if (chave === "vrr") {
      if (typeof valor === "boolean") {
        linhaProps.control = ({ labelId, descriptionId }: LinhaIds) => (
          <Chave
            checked={valor}
            onChange={(novo) => onChange?.("vrr", novo)}
            aria-labelledby={labelId}
            aria-describedby={descriptionId}
          />
        );
      } else {
        const listaOpcoes = opcoes?.vrr ?? [];
        linhaProps.control = ({ labelId, descriptionId }: LinhaIds) => (
          <CampoDeSelecao
            label={tituloLinha}
            options={listaOpcoes}
            value={String(valor)}
            onChange={(novo) => onChange?.("vrr", novo)}
            aria-labelledby={labelId}
            aria-describedby={descriptionId}
          />
        );
      }
      return <LinhaDeConfiguracao key={chave} {...linhaProps} />;
    }

    if (onCliqueLinha) {
      linhaProps.onClick = () => onCliqueLinha(chave);
      if (typeof valor === "string" && valor) {
        linhaProps.value = valor;
      }
      return <LinhaDeConfiguracao key={chave} {...linhaProps} />;
    }

    linhaProps.control = ({ labelId, descriptionId }: LinhaIds) => (
      <input
        type="text"
        className={styles["campoTexto"]}
        value={typeof valor === "string" ? valor : ""}
        placeholder={chave === "variaveisDeAmbiente" ? rotulos.nenhuma : rotulos.nenhum}
        onChange={(e) => onChange?.(chave, e.target.value)}
        aria-labelledby={labelId}
        aria-describedby={descriptionId}
      />
    );
    return <LinhaDeConfiguracao key={chave} {...linhaProps} />;
  };

  const temLinhasTela = CHAVES_TELA.some((ch) => valores[ch] !== undefined && valores[ch] !== null);
  const temLinhasSom = CHAVES_SOM.some((ch) => valores[ch] !== undefined && valores[ch] !== null);
  const temLinhasAbrir = CHAVES_ABRIR.some((ch) => valores[ch] !== undefined && valores[ch] !== null);

  const descricaoUsarTudo =
    typeof rotulos.descricaoUsarTudoDoTako === "function"
      ? rotulos.descricaoUsarTudoDoTako(mudancas)
      : (rotulos.descricaoUsarTudoDoTako ??
        (mudancas > 0
          ? `Volta as ${mudancas} ${mudancas === 1 ? singularMudanca : pluralMudancas} ao padrao`
          : (rotulos.nenhumaMudanca ?? "Nenhuma mudanca ao padrao")));

  const dangerLinhaProps: LinhaPropsBase = {
    danger: true,
    title: rotulos.usarTudoDoTako,
    description: descricaoUsarTudo,
  };
  if (onResetarAoPadrao) {
    dangerLinhaProps.onClick = onResetarAoPadrao;
  }

  const conteudo = (
    <div className={styles["raiz"]}>
      {temLinhasTela && (
        <GrupoDeConfiguracao title={rotulos.grupoTela}>
          {CHAVES_TELA.map((ch) => renderLinha(ch))}
        </GrupoDeConfiguracao>
      )}

      {temLinhasSom && (
        <GrupoDeConfiguracao title={rotulos.grupoSom}>
          {CHAVES_SOM.map((ch) => renderLinha(ch))}
        </GrupoDeConfiguracao>
      )}

      {temLinhasAbrir && (
        <GrupoDeConfiguracao title={rotulos.grupoAbrirOJogo}>
          {CHAVES_ABRIR.map((ch) => renderLinha(ch))}
        </GrupoDeConfiguracao>
      )}

      <GrupoDeConfiguracao>
        <div className={styles["linhaPerigo"]}>
          <LinhaDeConfiguracao {...dangerLinhaProps} />
        </div>
      </GrupoDeConfiguracao>
    </div>
  );

  if (inline) {
    return (
      <aside className={cx(styles["painelInline"], className)}>
        <header className={styles["cabecalhoInline"]}>
          {capaUrl && <img src={capaUrl} alt="" className={styles["capaInline"]} />}
          <div className={styles["titulosInline"]}>
            <h3 className={styles["tituloInline"]}>{rotulos.titulo}</h3>
            <span className={styles["subtituloInline"]}>{subtituloPainel}</span>
          </div>
          {onClose && (
            <button
              type="button"
              className={styles["botaoFecharInline"]}
              aria-label={rotulos.fechar}
              title={rotulos.fechar}
              onClick={onClose}
            >
              <Icon name="x" />
            </button>
          )}
        </header>
        <div className={styles["corpoInline"]}>{conteudo}</div>
      </aside>
    );
  }

  return (
    <PainelLateral
      open={open}
      onClose={onClose ?? (() => {})}
      wide
      title={rotulos.titulo}
      count={subtituloPainel}
      closeLabel={rotulos.fechar}
    >
      {conteudo}
    </PainelLateral>
  );
}
