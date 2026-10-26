import kana from "../../assets/brand/tako-kana.png";
import horizontal from "../../assets/brand/tako-logo-horizontal.png";
import mark from "../../assets/brand/tako-mark.png";
import wordmark from "../../assets/brand/tako-wordmark.png";
import styles from "./Marca.module.css";

interface MarcaProps {
  /** `bar` = polvo, palavra e kana como na barra de cima; `horizontal` = as três peças numa imagem só. */
  variant?: "bar" | "horizontal";
}

/** Marca do Tako. Só sobre fundo escuro (`canvas`, `titlebar`); nunca recolorir nem pôr sobre arte de jogo. */
export function Marca({ variant = "bar" }: MarcaProps) {
  if (variant === "horizontal") {
    return <img className={styles["horizontal"]} src={horizontal} alt="TAKO" />;
  }
  return (
    <span className={styles["bar"]} role="img" aria-label="TAKO">
      <img className={styles["mark"]} src={mark} alt="" />
      <span className={styles["text"]}>
        <img className={styles["word"]} src={wordmark} alt="" />
        <img className={styles["kana"]} src={kana} alt="" />
      </span>
    </span>
  );
}
