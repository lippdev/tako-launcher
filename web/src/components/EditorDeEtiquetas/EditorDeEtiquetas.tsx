import { useId, useLayoutEffect, useMemo, useRef, useState } from "react";
import { cx } from "../../lib/cx";
import { Portal } from "../../lib/Portal";
import { placeMenu, rootZoom } from "../../lib/position";
import { Icon } from "../Icon/Icon";
import { MAX_NOME, sugerir, type SugestaoDeEtiqueta } from "./sugestoes";
import styles from "./EditorDeEtiquetas.module.css";

export type { SugestaoDeEtiqueta };

export interface EditorDeEtiquetasLabels {
  /** Nome da lista de etiquetas para leitores de tela. */
  list: string;
  /** O texto do chip tracejado ("Etiqueta"; o + vem do ícone). */
  add: string;
  /** Nome do campo quando ele abre. */
  field: string;
  /** Cabeçalho das sugestões. */
  title: string;
  /** Nome do ✕ de cada chip ("Tirar Zerado"). */
  remove: (nome: string) => string;
  /** A linha que cria o que foi digitado (`Criar "Coop"`). */
  create: (nome: string) => string;
  /** Quantos jogos têm a etiqueta ("3 jogos"). */
  games: (n: number) => string;
}

export interface EditorDeEtiquetasProps {
  /** As etiquetas do jogo. */
  tags: readonly string[];
  /** Todas as etiquetas que existem na biblioteca, com quantos jogos têm cada uma. */
  suggestions: readonly SugestaoDeEtiqueta[];
  labels: EditorDeEtiquetasLabels;
  /** Aplica uma etiqueta que já existe ou cria uma nova com este nome. */
  onAdd: (nome: string) => void;
  onRemove: (nome: string) => void;
  /** Tamanho dos chips (`md` = 32, `sm` = 26, para a seção Detalhes). */
  size?: "md" | "sm";
  /** Onde os chips se juntam quando sobra espaço: no começo (padrão) ou no fim, como na seção Detalhes. */
  align?: "start" | "end";
  className?: string | undefined;
}

/**
 * Editor das etiquetas de um jogo (README do EtiquetasDoJogo, LIB-002): chips com ✕ (alvo de 24), o chip tracejado
 * "+ Etiqueta" que vira campo, e as sugestões embaixo com a quantidade de jogos e `Criar "…"`. Enter escolhe a
 * linha marcada (↑ ↓ mudam), Esc fecha o campo e Backspace no campo vazio tira a última etiqueta.
 */
export function EditorDeEtiquetas({
  tags,
  suggestions,
  labels,
  onAdd,
  onRemove,
  size = "md",
  align = "start",
  className,
}: EditorDeEtiquetasProps) {
  const [aberto, setAberto] = useState(false);
  const [texto, setTexto] = useState("");
  const [marcada, setMarcada] = useState(0);
  // Enter num campo vazio só escolhe se a pessoa já andou pela lista: senão aplicaria a primeira sem ninguém pedir.
  const [andou, setAndou] = useState(false);
  const campo = useRef<HTMLInputElement>(null);
  const caixa = useRef<HTMLSpanElement>(null);
  const lista = useRef<HTMLDivElement>(null);
  const listaId = useId();

  const { existentes, criar } = useMemo(() => sugerir(suggestions, tags, texto), [suggestions, tags, texto]);
  const linhas = [...existentes.map((e) => e.nome), ...(criar ? [criar] : [])];
  const atual = Math.min(marcada, Math.max(0, linhas.length - 1));
  const mostrarLista = aberto && linhas.length > 0;
  const small = size === "sm";

  // A lista fica junto ao campo, dentro da janela, e acompanha a janela e a rolagem.
  useLayoutEffect(() => {
    const menu = lista.current;
    const ancora = caixa.current;
    if (!mostrarLista || !menu || !ancora) return;
    const posicionar = () => {
      const zoom = rootZoom();
      const rect = ancora.getBoundingClientRect();
      const margem = 8 * zoom;
      const viewport = { width: window.innerWidth, height: window.innerHeight };
      menu.style.maxHeight = "";
      const natural = { width: menu.offsetWidth * zoom, height: menu.offsetHeight * zoom };
      const maxima = viewport.height - 2 * margem;
      if (natural.height > maxima) {
        menu.style.maxHeight = `${String(maxima / zoom)}px`;
        natural.height = maxima;
      }
      const lugar = placeMenu(rect, natural, viewport, { align: "start", gap: 6 * zoom, margin: margem });
      menu.style.left = `${String(lugar.left / zoom)}px`;
      menu.style.top = `${String(lugar.top / zoom)}px`;
    };
    posicionar();
    window.addEventListener("resize", posicionar);
    window.addEventListener("scroll", posicionar, true);
    return () => {
      window.removeEventListener("resize", posicionar);
      window.removeEventListener("scroll", posicionar, true);
    };
  }, [mostrarLista, linhas.length]);

  const abrir = () => {
    setAberto(true);
    // O campo só existe depois deste desenho.
    requestAnimationFrame(() => campo.current?.focus());
  };
  const fechar = () => {
    setAberto(false);
    setTexto("");
    setMarcada(0);
    setAndou(false);
  };
  const escolher = (nome: string) => {
    onAdd(nome);
    setTexto("");
    setMarcada(0);
    setAndou(false);
    campo.current?.focus();
  };

  return (
    <div
      role="list"
      aria-label={labels.list}
      data-alinhamento={align === "end" ? "fim" : "inicio"}
      className={cx(styles["raiz"], className)}
    >
      {tags.map((nome) => (
        <span key={nome} role="listitem" className={cx(styles["chip"], small && styles["sm"])}>
          <span className={styles["nome"]}>{nome}</span>
          <button
            type="button"
            className={styles["tirar"]}
            aria-label={labels.remove(nome)}
            title={labels.remove(nome)}
            onClick={() => {
              onRemove(nome);
            }}
          >
            <Icon name="x" size="sm" />
          </button>
        </span>
      ))}
      {aberto ? (
        <span ref={caixa} role="listitem" className={cx(styles["chip"], styles["adicionar"], small && styles["sm"])}>
          <Icon name="plus" size="sm" />
          <input
            ref={campo}
            type="text"
            className={styles["campo"]}
            value={texto}
            maxLength={MAX_NOME}
            placeholder={labels.add}
            aria-label={labels.field}
            role="combobox"
            aria-expanded={mostrarLista}
            aria-controls={listaId}
            aria-autocomplete="list"
            aria-activedescendant={mostrarLista ? `${listaId}-${String(atual)}` : undefined}
            onChange={(event) => {
              setTexto(event.target.value);
              setMarcada(0);
              setAndou(false);
            }}
            onBlur={fechar}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                event.stopPropagation();
                fechar();
              } else if (event.key === "Enter") {
                event.preventDefault();
                const nome = linhas[atual];
                if (nome && (texto !== "" || andou)) escolher(nome);
              } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
                event.preventDefault();
                const passo = event.key === "ArrowDown" ? 1 : -1;
                setAndou(true);
                setMarcada((atual + passo + linhas.length) % Math.max(1, linhas.length));
              } else if (event.key === "Backspace" && texto === "") {
                const ultima = tags[tags.length - 1];
                if (ultima) onRemove(ultima);
              }
            }}
          />
        </span>
      ) : (
        <button type="button" role="listitem" className={cx(styles["chip"], styles["adicionar"], small && styles["sm"])} onClick={abrir}>
          <Icon name="plus" size="sm" />
          {labels.add}
        </button>
      )}
      {mostrarLista && (
        <Portal>
          <div ref={lista} id={listaId} role="listbox" aria-label={labels.title} className={styles["sugestoes"]}>
            <div role="presentation" className={styles["titulo"]}>
              {labels.title}
            </div>
            {existentes.map((sugestao, i) => (
              <div
                key={sugestao.nome}
                id={`${listaId}-${String(i)}`}
                role="option"
                aria-selected={i === atual}
                className={styles["opcao"]}
                // O foco fica no campo: o clique escolhe sem tirá-lo de lá.
                onMouseDown={(event) => {
                  event.preventDefault();
                }}
                onMouseEnter={() => {
                  setMarcada(i);
                  setAndou(true);
                }}
                onClick={() => {
                  escolher(sugestao.nome);
                }}
              >
                <span className={styles["rotuloOpcao"]}>{sugestao.nome}</span>
                <span className={styles["contagem"]}>{labels.games(sugestao.jogos)}</span>
              </div>
            ))}
            {criar && (
              <>
                {existentes.length > 0 && <div role="separator" className={styles["separador"]} />}
                <div
                  id={`${listaId}-${String(existentes.length)}`}
                  role="option"
                  aria-selected={existentes.length === atual}
                  className={styles["opcao"]}
                  onMouseDown={(event) => {
                    event.preventDefault();
                  }}
                  onMouseEnter={() => {
                    setMarcada(existentes.length);
                    setAndou(true);
                  }}
                  onClick={() => {
                    escolher(criar);
                  }}
                >
                  <Icon name="plus" size="sm" />
                  <span className={styles["rotuloOpcao"]}>{labels.create(criar)}</span>
                </div>
              </>
            )}
          </div>
        </Portal>
      )}
    </div>
  );
}
