import { useRef, type ReactNode } from "react";
import { cx } from "../lib/cx";
import { usePresence } from "../lib/presence";
import styles from "./Vista.module.css";

interface VistaProps {
  /** A vista de que o título fala. */
  title: string;
  active: boolean;
  children?: ReactNode;
  /**
   * A vista cuida da própria rolagem e do próprio título (Configurações: lista de categorias fixa à esquerda,
   * conteúdo que rola à direita). Sem a margem da página nem o título padrão.
   */
  bare?: boolean;
}

/**
 * Uma vista do app (TrocaDeTela, MovimentoBase): a que sai some com escala 0,985 e opacidade (450 / 550 ms) e a
 * que entra faz o caminho de volta, uma por cima da outra. Só `transform` e `opacity` animam. Fora do ar a vista
 * sai do DOM, depois da saída.
 */
export function Vista({ title, active, children, bare = false }: VistaProps) {
  const ref = useRef<HTMLElement>(null);
  const { mounted, visible, animating } = usePresence(active, ref);
  if (!mounted) return null;
  return (
    <section
      ref={ref}
      aria-label={title}
      // A que está saindo não recebe clique, foco nem leitor de tela.
      inert={!active}
      data-visible={visible}
      data-animating={animating}
      className={cx(styles["view"], bare && styles["bare"])}
    >
      {bare ? (
        children
      ) : (
        // TODO: as telas de verdade entram aqui, cada uma na sua tarefa; por enquanto só o título.
        <div className={styles["page"]}>
          <h1 className="page-title">{title}</h1>
          {children}
        </div>
      )}
    </section>
  );
}
