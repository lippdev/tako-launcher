import { useState } from "react";
import { cx } from "../../lib/cx";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { MenuDeContexto, type MenuEntry } from "../MenuDeContexto/MenuDeContexto";
import styles from "./AcoesDoJogo.module.css";

/** O estado do primário (JogarEAbrir): Jogar → Abrindo… → Voltar ao jogo; não instalado; lançamento futuro. */
export type EstadoDoPrimario = "jogar" | "abrindo" | "jogando" | "nao-instalado" | "em-breve";

export interface AcoesDoJogoLabels {
  jogar: string;
  abrindo: string;
  voltarAoJogo: string;
  encerrar: string;
  instalar: string;
  emBreve: string;
  favoritar: string;
  desfavoritar: string;
  gerenciar: string;
  configuracoes: string;
}

export interface AcoesDoJogoProps {
  estado: EstadoDoPrimario;
  favorito: boolean;
  labels: AcoesDoJogoLabels;
  onJogar?: (() => void) | undefined;
  /** Jogando: volta para a janela do jogo. */
  onVoltarAoJogo?: (() => void) | undefined;
  /** Jogando, ou abrindo há tempo demais (cancelar): encerra o jogo. Sem ele o ✕ não aparece. */
  onEncerrar?: (() => void) | undefined;
  onAlternarFavorito: () => void;
  /** Escolher "Configurações do jogo" no menu Gerenciar. Sem ele a entrada fica desabilitada. */
  onConfiguracoes?: (() => void) | undefined;
  /** Outras entradas do menu Gerenciar, depois de "Configurações do jogo" (Fontes e IDs, Editar, Remover). */
  entradasExtras?: readonly MenuEntry[] | undefined;
  /** Só o primário (o destaque da Visão geral): sem ★ nem Gerenciar. */
  compacto?: boolean;
  className?: string | undefined;
}

/**
 * As ações do jogo da biblioteca, embaixo à direita do destaque (README do AcoesDoJogo): ★ favorito · Gerenciar ⚙ ·
 * primário. O primário muda com o estado (JogarEAbrir, EstadoJogando, EstadoNaoInstalado).
 */
export function AcoesDoJogo({
  estado,
  favorito,
  labels,
  onJogar,
  onVoltarAoJogo,
  onEncerrar,
  onAlternarFavorito,
  onConfiguracoes,
  entradasExtras,
  compacto = false,
  className,
}: AcoesDoJogoProps) {
  const [menuAberto, setMenuAberto] = useState(false);
  const [ancora, setAncora] = useState<HTMLButtonElement | null>(null);

  const entradas: readonly MenuEntry[] = [
    {
      id: "favorito",
      label: favorito ? labels.desfavoritar : labels.favoritar,
      icon: "star",
      onSelect: onAlternarFavorito,
    },
    { type: "separator" },
    // Sem o ouvinte (a galeria de componentes) a entrada fica à vista e desligada.
    {
      id: "configuracoes",
      label: labels.configuracoes,
      icon: "sliders",
      disabled: onConfiguracoes === undefined,
      onSelect: () => {
        onConfiguracoes?.();
      },
    },
    ...(entradasExtras ?? []),
  ];

  return (
    <div className={cx(styles["acoes"], className)}>
      {!compacto && (
        <>
          <BotaoDesktop
            variant="square"
            icon="star"
            on={favorito}
            tone="warning"
            aria-label={favorito ? labels.desfavoritar : labels.favoritar}
            title={favorito ? labels.desfavoritar : labels.favoritar}
            onClick={onAlternarFavorito}
          />
          <BotaoDesktop
            ref={setAncora}
            variant="secondary"
            icon="gear"
            data-command-menu=""
            aria-haspopup="menu"
            aria-expanded={menuAberto}
            onClick={() => {
              setMenuAberto((aberto) => !aberto);
            }}
          >
            {labels.gerenciar}
          </BotaoDesktop>
          <MenuDeContexto
            open={menuAberto}
            onClose={() => {
              setMenuAberto(false);
            }}
            anchor={ancora}
            entries={entradas}
            label={labels.gerenciar}
          />
        </>
      )}
      {(estado === "jogando" || estado === "abrindo") && onEncerrar && (
        <BotaoDesktop
          variant="square"
          icon="x"
          className={styles["encerrar"]}
          aria-label={labels.encerrar}
          title={labels.encerrar}
          onClick={onEncerrar}
        />
      )}
      <Primario estado={estado} labels={labels} onJogar={onJogar} onVoltarAoJogo={onVoltarAoJogo} />
    </div>
  );
}

function Primario({
  estado,
  labels,
  onJogar,
  onVoltarAoJogo,
}: Pick<AcoesDoJogoProps, "estado" | "labels" | "onJogar" | "onVoltarAoJogo">) {
  switch (estado) {
    case "jogando":
      return (
        <BotaoDesktop icon="gamepad" onClick={onVoltarAoJogo}>
          {labels.voltarAoJogo}
        </BotaoDesktop>
      );
    case "abrindo":
      return (
        <BotaoDesktop icon="play" iconFilled disabled aria-busy="true">
          {labels.abrindo}
        </BotaoDesktop>
      );
    case "nao-instalado":
      // TODO(instalação): o núcleo ainda não instala jogos pela ponte; o botão aparece (EstadoNaoInstalado) e fica
      // desligado até existir a chamada.
      return (
        <BotaoDesktop icon="download" disabled>
          {labels.instalar}
        </BotaoDesktop>
      );
    case "em-breve":
      return <BotaoDesktop disabled>{labels.emBreve}</BotaoDesktop>;
    case "jogar":
      return (
        <BotaoDesktop icon="play" iconFilled onClick={onJogar}>
          {labels.jogar}
        </BotaoDesktop>
      );
  }
}
