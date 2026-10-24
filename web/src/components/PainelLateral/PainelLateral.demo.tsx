import { useState } from "react";
import { cx } from "../../lib/cx";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../LinhaDeConfiguracao/LinhaDeConfiguracao";
import { Chave } from "../Chave/Chave";
import { PainelLateral } from "./PainelLateral";
import styles from "./PainelLateral.demo.module.css";

export const title = "PainelLateral";
export const preview = "PainelLateral";

const ITEMS = [
  { name: "Marvel's Spider-Man 2", price: "R$ 59,99" },
  { name: "Horizon Forbidden West", price: "R$ 38,99" },
];

export function Demo() {
  const [open, setOpen] = useState<"cart" | "game" | null>(null);
  const [fullscreen, setFullscreen] = useState(true);
  const close = () => {
    setOpen(null);
  };
  return (
    <div className={styles["buttons"]}>
      <button
        type="button"
        className={styles["trigger"]}
        onClick={() => {
          setOpen("cart");
        }}
      >
        Abrir o carrinho (400)
      </button>
      <button
        type="button"
        className={styles["trigger"]}
        onClick={() => {
          setOpen("game");
        }}
      >
        Abrir configurações do jogo (480)
      </button>
      <p className="label-sm">Esc fecha, Tab fica preso dentro, o foco volta para o botão que abriu.</p>

      <PainelLateral open={open === "cart"} onClose={close} title="Seu carrinho" count="2 itens" closeLabel="Fechar" footer={
        <>
          <div className={styles["totals"]}>
            <span>Preço</span>
            <span>R$ 98,98</span>
          </div>
          <div className={cx(styles["totals"], styles["discount"])}>
            <span>Descontos</span>
            <span>-R$ 21,00</span>
          </div>
          <div className={cx(styles["totals"], styles["subtotal"])}>
            <span>Subtotal</span>
            <span>R$ 77,98</span>
          </div>
          <button type="button" className={styles["primary"]}>
            Finalizar compra
          </button>
        </>
      }>
        <ul className={styles["list"]}>
          {ITEMS.map((item) => (
            <li key={item.name} className={styles["item"]}>
              <div className={styles["cover"]} />
              <div className={styles["itemText"]}>
                <strong>{item.name}</strong>
                <span className={styles["price"]}>{item.price}</span>
                <button type="button" className={styles["remove"]}>
                  Remover
                </button>
              </div>
            </li>
          ))}
        </ul>
      </PainelLateral>

      <PainelLateral open={open === "game"} onClose={close} wide title="Configurações deste jogo" count="1 mudança" closeLabel="Fechar">
        <GrupoDeConfiguracao title="Tela">
          <LinhaDeConfiguracao
            title="Iniciar em tela cheia"
            description="Abre o jogo ocupando a tela toda"
            control={({ labelId, descriptionId }) => (
              <Chave checked={fullscreen} onChange={setFullscreen} aria-labelledby={labelId} aria-describedby={descriptionId} />
            )}
          />
        </GrupoDeConfiguracao>
        <div className={styles["gap"]} />
        <GrupoDeConfiguracao>
          <LinhaDeConfiguracao danger title="Usar tudo do TAKO" description="Descarta as mudanças feitas só para este jogo" />
        </GrupoDeConfiguracao>
      </PainelLateral>
    </div>
  );
}

