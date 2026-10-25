import { useState, type MouseEvent } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { MenuDeContexto, type MenuEntry } from "./MenuDeContexto";

export const title = "MenuDeContexto";
export const preview = "MenuDeContexto";

/** Botão que abre um menu; o último escolhido aparece ao lado (a escolha fecha o menu e devolve o foco ao botão). */
function Gatilho({ texto, title: cabecalho, entries }: { texto: string; title?: string; entries: readonly MenuEntry[] }) {
  const [open, setOpen] = useState(false);
  const [button, setButton] = useState<HTMLButtonElement | null>(null);
  return (
    <>
      <BotaoDesktop
        ref={setButton}
        variant="outlined"
        trailingIcon="chevron-down"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => {
          setOpen((v) => !v);
        }}
      >
        {texto}
      </BotaoDesktop>
      <MenuDeContexto
        open={open}
        onClose={() => {
          setOpen(false);
        }}
        anchor={button}
        entries={entries}
        {...(cabecalho ? { title: cabecalho } : { label: texto })}
      />
    </>
  );
}

const noop = () => undefined;

const perfil: MenuEntry[] = [
  { id: "perfil", label: "Meu perfil", icon: "user", onSelect: noop },
  { id: "familia", label: "Família", icon: "users", onSelect: noop },
  { id: "config", label: "Configurações", icon: "gear", onSelect: noop },
  { type: "separator" },
  { id: "sair", label: "Sair", icon: "x", danger: true, onSelect: noop },
];

const situacao: MenuEntry[] = [
  { id: "jogando", label: "Jogando", color: "var(--success)", kind: "radio", checked: true, onSelect: noop },
  { id: "pausado", label: "Pausado", color: "var(--warning)", kind: "radio", onSelect: noop },
  { id: "zerado", label: "Zerado", color: "var(--accent)", kind: "radio", onSelect: noop },
  { id: "abandonado", label: "Abandonado", color: "var(--text-dim)", kind: "radio", disabled: true, onSelect: noop },
];

const gerenciar: MenuEntry[] = [
  { type: "header", label: "Arquivos" },
  { id: "pasta", label: "Abrir pasta do jogo", icon: "folder", onSelect: noop },
  { id: "verificar", label: "Verificar arquivos", icon: "shield", onSelect: noop },
  { type: "header", label: "Biblioteca" },
  { id: "capa", label: "Trocar capa", icon: "image", onSelect: noop },
  { id: "colecao", label: "Adicionar à coleção", icon: "books", onSelect: noop },
  { type: "separator" },
  { id: "remover", label: "Remover da biblioteca", icon: "trash", danger: true, onSelect: noop },
];

const ordem: MenuEntry[] = [
  { id: "nome", label: "Nome", kind: "radio", checked: true, onSelect: noop },
  { id: "recentes", label: "Jogados recentemente", kind: "radio", onSelect: noop },
  { id: "tempo", label: "Tempo de jogo", kind: "radio", onSelect: noop },
  { id: "instalacao", label: "Data de instalação", kind: "radio", onSelect: noop },
];

function BotaoDireito() {
  const [point, setPoint] = useState<{ x: number; y: number } | null>(null);
  return (
    <>
      <div
        tabIndex={0}
        role="group"
        aria-label="Área de teste do menu de botão direito"
        style={{
          display: "grid",
          placeItems: "center",
          width: 280,
          height: 64,
          border: "1px dashed var(--line-strong)",
          borderRadius: "var(--radius-row)",
          color: "var(--text-muted)",
          fontSize: 13,
        }}
        onContextMenu={(event: MouseEvent) => {
          event.preventDefault();
          setPoint({ x: event.clientX, y: event.clientY });
        }}
      >
        Clique com o botão direito aqui
      </div>
      <MenuDeContexto
        open={point !== null}
        onClose={() => {
          setPoint(null);
        }}
        anchor={point}
        label="Ações do jogo"
        entries={perfil}
      />
    </>
  );
}

export function Demo() {
  return (
    <div>
      <DemoGrupo titulo="Perfil — itens com ícone de 15, separador e perigo (abre 6 abaixo, alinhado à direita)">
        <Gatilho texto="Perfil" entries={perfil} />
        <Gatilho texto="Perfil, com título" title="Alyx" entries={perfil} />
      </DemoGrupo>
      <DemoGrupo titulo="Situação do jogo — ponto de cor, ✓ no escolhido, item desabilitado">
        <Gatilho texto="Situação" title="Situação" entries={situacao} />
      </DemoGrupo>
      <DemoGrupo titulo="Gerenciar — cabeçalhos em caixa alta">
        <Gatilho texto="Gerenciar" entries={gerenciar} />
      </DemoGrupo>
      <DemoGrupo titulo="Opções da lista — título da linha e ✓ na atual">
        <Gatilho texto="Ordenar por" title="Ordenar por" entries={ordem} />
      </DemoGrupo>
      <DemoGrupo titulo="Botão direito — abre no ponto do clique e se ajusta à janela">
        <BotaoDireito />
      </DemoGrupo>
    </div>
  );
}
