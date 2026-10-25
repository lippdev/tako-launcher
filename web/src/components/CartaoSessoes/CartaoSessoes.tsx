import { cx } from "../../lib/cx";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./CartaoSessoes.module.css";

export interface DadoDaBarra {
  id: string;
  icone: IconName;
  label: string;
  valor: string;
  /** Etiqueta colorida antes do valor (a nota do Metacritic): o número, e a cor pela faixa. */
  nota?: { valor: number; faixa: "good" | "mixed" | "bad" };
}

interface BarraProps {
  /** Os dados que o núcleo respondeu (Última sessão, Tempo de jogo, Metacritic). Sem nenhum, a barra não existe. */
  dados: readonly DadoDaBarra[];
  /** Nome da barra para leitores de tela. */
  "aria-label": string;
  className?: string | undefined;
}

/**
 * A barra de dados sobre a arte (README do CartaoSessoes): ícone de 20, rótulo 12/700 em caixa alta e valor 15/600,
 * separados por linhas de 1. A sombra de texto garante a leitura sobre qualquer arte.
 */
export function BarraDeDados({ dados, "aria-label": label, className }: BarraProps) {
  if (dados.length === 0) return null;
  return (
    <div className={cx(styles["barra"], className)} role="group" aria-label={label}>
      {dados.map((dado) => (
        <div key={dado.id} className={styles["item"]}>
          <Icon name={dado.icone} />
          <div>
            <small className={styles["rotulo"]}>{dado.label}</small>
            <b className={styles["valor"]}>
              {dado.nota && (
                <span className={cx(styles["nota"], styles[dado.nota.faixa])}>{dado.nota.valor}</span>
              )}
              {dado.valor}
            </b>
          </div>
        </div>
      ))}
    </div>
  );
}

export interface SessaoDoJogo {
  /** Quando começou ("23/10, 16:17"). */
  inicio: string;
  /** Quanto durou ("42 min"). */
  duracao: string;
}

interface ListaProps {
  title: string;
  sessoes: readonly SessaoDoJogo[];
  className?: string | undefined;
}

/** As sessões mais recentes numa caixa de linhas alternadas, como os Detalhes. Sem sessão, não existe. */
export function ListaDeSessoes({ title, sessoes, className }: ListaProps) {
  if (sessoes.length === 0) return null;
  return (
    <section className={cx(styles["lista"], className)} aria-label={title}>
      <h3 className={styles["titulo"]}>{title}</h3>
      {sessoes.map((sessao, i) => (
        <div key={i} className={styles["sessao"]}>
          <span className={styles["inicio"]}>{sessao.inicio}</span>
          <strong className={styles["duracao"]}>{sessao.duracao}</strong>
        </div>
      ))}
    </section>
  );
}

export type CartaoSessoesProps = ({ variante?: "barra" } & BarraProps) | ({ variante: "lista" } & ListaProps);

/** As sessões do jogo: a barra de dados do destaque (`barra`) ou a lista das últimas partidas (`lista`). */
export function CartaoSessoes(props: CartaoSessoesProps) {
  return props.variante === "lista" ? <ListaDeSessoes {...props} /> : <BarraDeDados {...props} />;
}
