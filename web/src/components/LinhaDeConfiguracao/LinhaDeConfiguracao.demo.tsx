import { useState } from "react";
import { Chave } from "../Chave/Chave";
import { ControleDeslizante } from "../ControleDeslizante/ControleDeslizante";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "./LinhaDeConfiguracao";

export const title = "LinhaDeConfiguracao";
export const preview = "LinhaDeConfiguracao";

// Botão contornado e seletor da prévia: marcadores simples. O BotaoDesktop e o CampoDeSelecao são da outra tarefa.
const outlined = {
  height: "var(--control-height)",
  padding: "0 var(--space-12)",
  background: "var(--settings-control)",
  border: "1px solid var(--line-strong)",
  borderRadius: "var(--radius-row)",
  fontWeight: 600,
  fontSize: 13,
  cursor: "pointer",
} as const;

export function Demo() {
  const [ask, setAsk] = useState(true);
  const [startup, setStartup] = useState(false);
  const [sounds, setSounds] = useState(true);
  const [volume, setVolume] = useState(60);
  return (
    <div style={{ display: "grid", gap: "var(--space-24)", maxWidth: 820 }}>
      <GrupoDeConfiguracao title="Modo">
        <LinhaDeConfiguracao
          title="Ir para a tela cheia"
          description="Interface para controle e TV. Atalho no desktop: F11"
          control={<button style={outlined}>Trocar</button>}
        />
        <LinhaDeConfiguracao
          title="Perguntar pela tela cheia ao achar um controle"
          description="Quando o TAKO abre no desktop com um controle ligado"
          control={({ labelId, descriptionId }) => (
            <Chave checked={ask} onChange={setAsk} aria-labelledby={labelId} aria-describedby={descriptionId} />
          )}
        />
        <LinhaDeConfiguracao title="Abrir em" control={<button style={outlined}>Janela</button>} />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title="Perfil, início e idioma">
        <LinhaDeConfiguracao
          title="Perfil"
          description="Cada pessoa tem sua biblioteca, favoritos, tempo de jogo e configurações"
          value="Principal"
          onClick={() => undefined}
        />
        <LinhaDeConfiguracao
          title="Tela cheia do Windows"
          description="Usar o TAKO como tela inicial da experiência de tela cheia do Windows"
          onClick={() => undefined}
        />
        <LinhaDeConfiguracao
          title="Abrir o TAKO ao entrar no Windows"
          description="Abre o TAKO assim que você entra no computador"
          control={({ labelId, descriptionId }) => (
            <Chave checked={startup} onChange={setStartup} aria-labelledby={labelId} aria-describedby={descriptionId} />
          )}
        />
        <LinhaDeConfiguracao title="Novidade" tag="Novo" description="Linha com etiqueta na cor de destaque" control={<span>Ligado</span>} />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title="Interface">
        <LinhaDeConfiguracao
          title="Sons da interface"
          description="Sons discretos ao navegar e confirmar"
          control={({ labelId, descriptionId }) => (
            <Chave checked={sounds} onChange={setSounds} aria-labelledby={labelId} aria-describedby={descriptionId} />
          )}
        />
        <LinhaDeConfiguracao
          title="Volume"
          description="Volume da interface e das notificações"
          control={({ labelId, descriptionId }) => (
            <ControleDeslizante
              value={volume}
              onChange={setVolume}
              format={(v) => `${v}%`}
              aria-labelledby={labelId}
              aria-describedby={descriptionId}
            />
          )}
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title="Perigo">
        <LinhaDeConfiguracao
          danger
          title="Usar tudo do TAKO"
          description="Descarta as mudanças feitas só para este jogo"
          control={<button style={outlined}>Restaurar</button>}
        />
      </GrupoDeConfiguracao>
    </div>
  );
}
