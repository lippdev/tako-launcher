import { useMemo, useState, type ReactNode } from "react";
import { cx } from "../../lib/cx";
import { PainelLateral } from "../PainelLateral/PainelLateral";
import { GrupoDeConfiguracao, LinhaDeConfiguracao, type LinhaIds } from "../LinhaDeConfiguracao/LinhaDeConfiguracao";
import { Chave } from "../Chave/Chave";
import { CampoDeSelecao, type OpcaoDeSelecao } from "../CampoDeSelecao/CampoDeSelecao";
import { Icon } from "../Icon/Icon";
import styles from "./ConfiguracoesDoJogo.module.css";

export type { OpcaoDeSelecao };

export type ChaveConfiguracaoJogo =
  | "usarTela"
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
  usarTela?: boolean;
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
  usarTela?: string;
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

export interface ConfiguracoesDoJogoLabels {
  title: string;
  close: string;
  screenGroup: string;
  soundGroup: string;
  launchGroup: string;
  /** "Usar a tela de jogo" (liga e desliga); sem ele a linha `usarTela` não aparece. */
  useScreen?: string;
  monitor: string;
  resolutionAndRate: string;
  hdr: string;
  vrr: string;
  audioOutput: string;
  arguments: string;
  workingDirectory: string;
  environmentVariables: string;
  beforeAndAfterCommands: string;
  useAllFromTako: string;
  useAllFromTakoDescription?: string | ((mudancas: number) => string);
  defaultPrefix?: string;
  none?: string;
  noneFem?: string;
  on?: string;
  off?: string;
  singularChange?: string;
  pluralChanges?: string;
  noChanges?: string;
}

export interface ConfiguracoesDoJogoProps {
  open?: boolean;
  onClose?: () => void;
  gameTitle: string;
  coverUrl?: string;
  values: ValoresConfiguracoesDoJogo;
  defaults?: Partial<ValoresConfiguracoesDoJogo>;
  options?: OpcoesConfiguracoesDoJogo;
  descriptions?: DescricoesConfiguracoesDoJogo;
  labels: ConfiguracoesDoJogoLabels;
  changesCount?: number;
  /**
   * Linhas que só mostram o valor, sem controle: o que o jogo guarda mas esta tela ainda não grava (comandos) ou o que
   * o sistema não deixa mudar (VRR sem suporte). O motivo vai na descrição da linha.
   */
  readOnly?: readonly ChaveConfiguracaoJogo[];
  /** O que dizer de uma linha de texto vazia ("Nenhum", "Pasta do executável"), no campo e em "Padrão: …". */
  emptyText?: Partial<Record<ChaveConfiguracaoJogo, string>>;
  inline?: boolean;
  onChange?: (chave: ChaveConfiguracaoJogo, valor: ValorConfiguracaoJogo) => void;
  onResetToDefault?: () => void;
  onLineClick?: (chave: ChaveConfiguracaoJogo) => void;
  className?: string;
}

const CHAVES_TELA: readonly ChaveConfiguracaoJogo[] = ["usarTela", "monitor", "resolucaoETaxa", "hdr", "vrr"];
const CHAVES_SOM: readonly ChaveConfiguracaoJogo[] = ["saidaDeAudio"];
const CHAVES_ABRIR: readonly ChaveConfiguracaoJogo[] = [
  "argumentos",
  "pastaDeTrabalho",
  "variaveisDeAmbiente",
  "comandosAntesEDepois",
];

const CHAVE_LABEL_MAP: Record<ChaveConfiguracaoJogo, keyof ConfiguracoesDoJogoLabels> = {
  usarTela: "useScreen",
  monitor: "monitor",
  resolucaoETaxa: "resolutionAndRate",
  hdr: "hdr",
  vrr: "vrr",
  saidaDeAudio: "audioOutput",
  argumentos: "arguments",
  pastaDeTrabalho: "workingDirectory",
  variaveisDeAmbiente: "environmentVariables",
  comandosAntesEDepois: "beforeAndAfterCommands",
};

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
  gameTitle,
  coverUrl,
  values,
  defaults,
  options,
  descriptions,
  labels,
  changesCount: changesCountProp,
  readOnly = [],
  emptyText,
  inline = false,
  onChange,
  onResetToDefault,
  onLineClick,
  className,
}: ConfiguracoesDoJogoProps) {
  const padraoPrefixo = labels.defaultPrefix ?? "Padrao: ";

  const formatarRotuloValorPadrao = (chave: ChaveConfiguracaoJogo, valorPadrao: ValorConfiguracaoJogo): string => {
    if (typeof valorPadrao === "boolean") {
      return valorPadrao ? (labels.on ?? "ligado") : (labels.off ?? "desligado");
    }
    const listaOpcoes = options?.[chave as keyof OpcoesConfiguracoesDoJogo];
    if (listaOpcoes) {
      const encontrada = listaOpcoes.find((o: OpcaoDeSelecao<string>) => o.value === String(valorPadrao));
      if (encontrada) {
        return encontrada.label;
      }
    }
    if (typeof valorPadrao === "string" && valorPadrao.length > 0) {
      return valorPadrao;
    }
    if (emptyText?.[chave]) {
      return emptyText[chave];
    }
    if (chave === "variaveisDeAmbiente") {
      return labels.noneFem ?? "nenhuma";
    }
    return labels.none ?? "nenhum";
  };

  const chavesMudadas = useMemo(() => {
    if (!defaults) return [];
    const chaves: ChaveConfiguracaoJogo[] = [];
    const todas: ChaveConfiguracaoJogo[] = [...CHAVES_TELA, ...CHAVES_SOM, ...CHAVES_ABRIR];
    for (const ch of todas) {
      const val = values[ch];
      const def = defaults[ch];
      if (val !== undefined && def !== undefined && val !== def) {
        chaves.push(ch);
      }
    }
    return chaves;
  }, [values, defaults]);

  const mudancas = changesCountProp !== undefined ? changesCountProp : chavesMudadas.length;

  const singularMudanca = labels.singularChange ?? "mudanca";
  const pluralMudancas = labels.pluralChanges ?? "mudancas";
  const nenhumaMudanca = labels.noChanges ?? "nenhuma mudanca";

  const textoMudancas =
    mudancas === 0
      ? nenhumaMudanca
      : mudancas === 1
        ? `1 ${singularMudanca}`
        : `${mudancas} ${pluralMudancas}`;

  const subtituloPainel = `${gameTitle} \u00b7 ${textoMudancas}`;

  /** O valor de uma linha só de leitura, em texto: o rótulo da opção, ligado/desligado ou o texto do vazio. */
  const textoDeLeitura = (chave: ChaveConfiguracaoJogo, valor: ValorConfiguracaoJogo): string => {
    if (typeof valor === "boolean") {
      return valor ? (labels.on ?? "ligado") : (labels.off ?? "desligado");
    }
    const opcao = options?.[chave as keyof OpcoesConfiguracoesDoJogo]?.find((o) => o.value === valor);
    return opcao?.label ?? (valor.trim() || formatarRotuloValorPadrao(chave, ""));
  };

  const renderLinha = (chave: ChaveConfiguracaoJogo): ReactNode => {
    const valor = values[chave];
    if (valor === undefined || valor === null) {
      return null;
    }

    const soLeitura = readOnly.includes(chave);
    const valorPadrao = defaults?.[chave];
    const mudou = !soLeitura && valorPadrao !== undefined && valor !== valorPadrao;

    let descricaoFinal: string | undefined = descriptions?.[chave];
    if (mudou && valorPadrao !== undefined) {
      descricaoFinal = `${padraoPrefixo}${formatarRotuloValorPadrao(chave, valorPadrao)}`;
    } else if (!descricaoFinal) {
      if (chave === "argumentos" || chave === "comandosAntesEDepois") {
        descricaoFinal = (typeof valor === "string" && valor.trim()) ? undefined : (labels.none ?? "Nenhum");
      } else if (chave === "variaveisDeAmbiente") {
        descricaoFinal = (typeof valor === "string" && valor.trim()) ? undefined : (labels.noneFem ?? "Nenhuma");
      }
    }

    const labelKey = CHAVE_LABEL_MAP[chave];
    const tituloLinha = (labelKey ? labels[labelKey] : undefined) as string ?? "";
    const linhaProps: LinhaPropsBase = { title: tituloLinha };
    if (descricaoFinal) {
      linhaProps.description = descricaoFinal;
    }

    if (soLeitura) {
      linhaProps.control = <span className={styles["valorLeitura"]}>{textoDeLeitura(chave, valor)}</span>;
      return <LinhaDeConfiguracao key={chave} {...linhaProps} />;
    }

    if (chave === "usarTela") {
      if (!labels.useScreen) return null;
      linhaProps.control = ({ labelId, descriptionId }: LinhaIds) => (
        <Chave
          checked={Boolean(valor)}
          onChange={(novo) => onChange?.("usarTela", novo)}
          aria-labelledby={labelId}
          aria-describedby={descriptionId}
        />
      );
      return <LinhaDeConfiguracao key={chave} {...linhaProps} />;
    }

    if (chave === "monitor") {
      const opcoesMonitor = options?.monitor;
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
      const listaOpcoes = options?.[chave] ?? [];
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
        const listaOpcoes = options?.vrr ?? [];
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

    if (onLineClick) {
      linhaProps.onClick = () => onLineClick(chave);
      if (typeof valor === "string" && valor) {
        linhaProps.value = valor;
      }
      return <LinhaDeConfiguracao key={chave} {...linhaProps} />;
    }

    linhaProps.control = ({ labelId, descriptionId }: LinhaIds) => (
      <CampoDaLinha
        value={typeof valor === "string" ? valor : ""}
        placeholder={emptyText?.[chave] ?? (chave === "variaveisDeAmbiente" ? labels.noneFem : labels.none)}
        onCommit={(novo) => onChange?.(chave, novo)}
        aria-labelledby={labelId}
        aria-describedby={descriptionId}
      />
    );
    return <LinhaDeConfiguracao key={chave} {...linhaProps} />;
  };

  const temLinhasTela = CHAVES_TELA.some((ch) => values[ch] !== undefined && values[ch] !== null);
  const temLinhasSom = CHAVES_SOM.some((ch) => values[ch] !== undefined && values[ch] !== null);
  const temLinhasAbrir = CHAVES_ABRIR.some((ch) => values[ch] !== undefined && values[ch] !== null);

  const descricaoUsarTudo =
    typeof labels.useAllFromTakoDescription === "function"
      ? labels.useAllFromTakoDescription(mudancas)
      : (labels.useAllFromTakoDescription ??
        (mudancas > 0
          ? `Volta as ${mudancas} ${mudancas === 1 ? singularMudanca : pluralMudancas} ao padrao`
          : (labels.noChanges ?? "Nenhuma mudanca ao padrao")));

  const dangerLinhaProps: LinhaPropsBase = {
    danger: true,
    title: labels.useAllFromTako,
    description: descricaoUsarTudo,
  };
  if (onResetToDefault) {
    dangerLinhaProps.onClick = onResetToDefault;
  }

  const conteudo = (
    <div className={styles["raiz"]}>
      {temLinhasTela && (
        <GrupoDeConfiguracao title={labels.screenGroup}>
          {CHAVES_TELA.map((ch) => renderLinha(ch))}
        </GrupoDeConfiguracao>
      )}

      {temLinhasSom && (
        <GrupoDeConfiguracao title={labels.soundGroup}>
          {CHAVES_SOM.map((ch) => renderLinha(ch))}
        </GrupoDeConfiguracao>
      )}

      {temLinhasAbrir && (
        <GrupoDeConfiguracao title={labels.launchGroup}>
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
          {coverUrl && <img src={coverUrl} alt="" className={styles["capaInline"]} />}
          <div className={styles["titulosInline"]}>
            <h3 className={styles["tituloInline"]}>{labels.title}</h3>
            <span className={styles["subtituloInline"]}>{subtituloPainel}</span>
          </div>
          {onClose && (
            <button
              type="button"
              className={styles["botaoFecharInline"]}
              aria-label={labels.close}
              title={labels.close}
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
      title={labels.title}
      count={subtituloPainel}
      closeLabel={labels.close}
    >
      {conteudo}
    </PainelLateral>
  );
}

interface CampoDaLinhaProps {
  value: string;
  placeholder: string | undefined;
  onCommit: (valor: string) => void;
  "aria-labelledby": string;
  "aria-describedby": string;
}

/**
 * Campo de texto de uma linha. Escreve num rascunho e só entrega o valor ao sair do campo ou com Enter: gravar a cada
 * tecla mandaria à ponte textos pela metade (`NOME` sem o `=valor`). Se o valor de fora mudar (a gravação falhou e
 * voltou o de antes), o rascunho o acompanha.
 */
function CampoDaLinha({ value, placeholder, onCommit, ...aria }: CampoDaLinhaProps) {
  const [rascunho, setRascunho] = useState(value);
  const [visto, setVisto] = useState(value);
  if (value !== visto) {
    setVisto(value);
    setRascunho(value);
  }
  return (
    <input
      type="text"
      className={styles["campoTexto"]}
      value={rascunho}
      placeholder={placeholder}
      onChange={(e) => {
        setRascunho(e.target.value);
      }}
      onBlur={() => {
        if (rascunho !== value) onCommit(rascunho);
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
      }}
      {...aria}
    />
  );
}
