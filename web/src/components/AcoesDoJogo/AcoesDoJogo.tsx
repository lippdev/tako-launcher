import { useState } from "react";
import { cx } from "../../lib/cx";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { MenuDeContexto, type MenuEntry } from "../MenuDeContexto/MenuDeContexto";
import styles from "./AcoesDoJogo.module.css";

/** O estado do primário (JogarEAbrir): Jogar → Abrindo… → Voltar ao jogo; não instalado; lançamento futuro. */
export type EstadoDoPrimario = "jogar" | "abrindo" | "jogando" | "nao-instalado" | "em-breve";

export interface AcoesDoJogoRotulos {
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
  rotulos: AcoesDoJogoRotulos;
  onJogar?: (() => void) | undefined;
  /** Jogando: volta para a janela do jogo. */
  onVoltarAoJogo?: (() => void) | undefined;
  /** Jogando: encerra o jogo. Sem ele o ✕ não aparece. */
  onEncerrar?: (() => void) | undefined;
  onAlternarFavorito: () => void;
  /** Escolher "Configurações do jogo" no menu Gerenciar. Sem ele a entrada fica desabilitada. */
  onConfiguracoes?: (() => void) | undefined;
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
  rotulos,
  onJogar,
  onVoltarAoJogo,
  onEncerrar,
  onAlternarFavorito,
  onConfiguracoes,
  compacto = false,
  className,
}: AcoesDoJogoProps) {
  const [menuAberto, setMenuAberto] = useState(false);
  const [ancora, setAncora] = useState<HTMLButtonElement | null>(null);

  const entradas: readonly MenuEntry[] = [
    {
      id: "favorito",
      label: favorito ? rotulos.desfavoritar : rotulos.favoritar,
      icon: "star",
      onSelect: onAlternarFavorito,
    },
    { type: "separator" },
    // TODO(WEB-12b): abre o painel de configurações do jogo; até lá a entrada fica à vista e desligada.
    {
      id: "configuracoes",
      label: rotulos.configuracoes,
      icon: "sliders",
      disabled: onConfiguracoes === undefined,
      onSelect: () => {
        onConfiguracoes?.();
      },
    },
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
            aria-label={favorito ? rotulos.desfavoritar : rotulos.favoritar}
            title={favorito ? rotulos.desfavoritar : rotulos.favoritar}
            onClick={onAlternarFavorito}
          />
          <BotaoDesktop
            ref={setAncora}
            variant="secondary"
            icon="gear"
            aria-haspopup="menu"
            aria-expanded={menuAberto}
            onClick={() => {
              setMenuAberto((aberto) => !aberto);
            }}
          >
            {rotulos.gerenciar}
          </BotaoDesktop>
          <MenuDeContexto
            open={menuAberto}
            onClose={() => {
              setMenuAberto(false);
            }}
            anchor={ancora}
            entries={entradas}
            label={rotulos.gerenciar}
          />
        </>
      )}
      {estado === "jogando" && onEncerrar && (
        <BotaoDesktop
          variant="square"
          icon="x"
          className={styles["encerrar"]}
          aria-label={rotulos.encerrar}
          title={rotulos.encerrar}
          onClick={onEncerrar}
        />
      )}
      <Primario estado={estado} rotulos={rotulos} onJogar={onJogar} onVoltarAoJogo={onVoltarAoJogo} />
    </div>
  );
}

function Primario({
  estado,
  rotulos,
  onJogar,
  onVoltarAoJogo,
}: Pick<AcoesDoJogoProps, "estado" | "rotulos" | "onJogar" | "onVoltarAoJogo">) {
  switch (estado) {
    case "jogando":
      return (
        <BotaoDesktop icon="gamepad" onClick={onVoltarAoJogo}>
          {rotulos.voltarAoJogo}
        </BotaoDesktop>
      );
    case "abrindo":
      return (
        <BotaoDesktop icon="play" iconFilled disabled aria-busy="true">
          {rotulos.abrindo}
        </BotaoDesktop>
      );
    case "nao-instalado":
      // TODO(instalação): o núcleo ainda não instala jogos pela ponte; o botão aparece (EstadoNaoInstalado) e fica
      // desligado até existir a chamada.
      return (
        <BotaoDesktop icon="download" disabled>
          {rotulos.instalar}
        </BotaoDesktop>
      );
    case "em-breve":
      return <BotaoDesktop disabled>{rotulos.emBreve}</BotaoDesktop>;
    case "jogar":
      return (
        <BotaoDesktop icon="play" iconFilled onClick={onJogar}>
          {rotulos.jogar}
        </BotaoDesktop>
      );
  }
}
