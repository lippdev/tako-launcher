import {
  forwardRef,
  useEffect,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type ReactNode,
} from "react";
import { Icon } from "../Icon/Icon";
import { cx } from "../../lib/cx";
import styles from "./AtividadeNaBarra.module.css";

export interface AtividadeNaBarraProps
  extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "onClick"> {
  /** Quantidade de atividades/downloads ativos. Se omitido ou <= 0, o badge fica oculto. */
  count?: number;
  /** Porcentagem geral ou do download ativo (0 a 100). */
  progress?: number | null;
  /** Se o painel está aberto (para aria-expanded e estado ativo). */
  open?: boolean;
  /** Callback acionado ao clicar no botão. */
  onClick?: () => void;
  onToggle?: () => void;
  /** Rótulo acessível explícito. Se omitido, é gerado conforme contagem e porcentagem. */
  "aria-label"?: string;
  /** Título do botão (tooltip nativo ou data-tip). */
  title?: string;
  /** Força o estado visual para testes na galeria (hover, focus, pressed). */
  "data-force"?: "hover" | "focus" | "pressed";
  children?: ReactNode;
}

/**
 * AtividadeNaBarra: botão de downloads da barra de ferramentas superior do TAKO.
 * Mostra ícone circular de download quando ocioso. Quando há atividades, exibe
 * contador com pulo suave (escala 1.35 em 400ms) e porcentagem opcional.
 */
export const AtividadeNaBarra = forwardRef<HTMLButtonElement, AtividadeNaBarraProps>(
  function AtividadeNaBarra(props, ref) {
    const {
      count = 0,
      progress = null,
      open = false,
      onClick,
      onToggle,
      "aria-label": ariaLabelProp,
      title = "Downloads",
      "data-force": dataForce,
      className,
      disabled,
      children,
      ...rest
    } = props;

    const totalAtividades = count;
    const temAtividade = totalAtividades > 0;
    const pct = progress;
    const painelAberto = open;

    // Animação de pulo (tk-pop) ao mudar a contagem de atividades
    const [pop, setPop] = useState(false);
    const anteriorRef = useRef(totalAtividades);

    useEffect(() => {
      if (temAtividade && totalAtividades !== anteriorRef.current) {
        setPop(true);
        const timer = setTimeout(() => {
          setPop(false);
        }, 400);
        return () => clearTimeout(timer);
      }
      anteriorRef.current = totalAtividades;
    }, [totalAtividades, temAtividade]);

    // Rótulo acessível obrigatório
    const rotuloAcessivel =
      ariaLabelProp ??
      (temAtividade
        ? pct != null
          ? `${title}: ${totalAtividades} ${totalAtividades === 1 ? "ativo" : "ativos"}, ${Math.round(pct)}%`
          : `${title}: ${totalAtividades} ${totalAtividades === 1 ? "ativo" : "ativos"}`
        : title);

    const handleClick = () => {
      if (disabled) return;
      (onClick ?? onToggle)?.();
    };

    return (
      <button
        ref={ref}
        type="button"
        className={cx(
          styles["btn"],
          painelAberto && styles["aberto"],
          className
        )}
        onClick={handleClick}
        disabled={disabled}
        aria-label={rotuloAcessivel}
        aria-haspopup="dialog"
        aria-expanded={painelAberto}
        title={title}
        data-tip={title}
        data-force={dataForce}
        {...rest}
      >
        <Icon name="download-circle" size="md" />
        {temAtividade && pct != null && (
          <span className={styles["pct"]} aria-hidden="true">
            {Math.round(pct)}%
          </span>
        )}
        {temAtividade && (
          <span
            className={cx(styles["badge"], pop && styles["pop"])}
            aria-hidden="true"
            onAnimationEnd={() => setPop(false)}
          >
            {totalAtividades}
          </span>
        )}
        {children}
      </button>
    );
  }
);
