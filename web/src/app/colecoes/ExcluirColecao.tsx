import { useState } from "react";
import type { Collection } from "../../bridge/generated/Collection";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { useT } from "../../i18n";
import styles from "./Colecoes.module.css";

interface ExcluirColecaoProps {
  /** A coleção a excluir; `null` fecha o diálogo. */
  collection: Pick<Collection, "name" | "criteria"> | null;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

/**
 * A pergunta "Excluir {nome}?" das coleções (manual ou automática), com o texto do Slint. Vale na lista da biblioteca
 * e em Configurações › Biblioteca.
 */
export function ExcluirColecao({ collection, busy, onCancel, onConfirm }: ExcluirColecaoProps) {
  const t = useT();
  // O diálogo sai com animação: a coleção continua no texto até ele sumir.
  const [shown, setShown] = useState(collection);
  if (collection && collection !== shown) setShown(collection);
  return (
    <Dialogo
      open={collection !== null}
      onClose={onCancel}
      size="s"
      title={t("settings.library.delete_title", { name: shown?.name ?? "" })}
      closeLabel={t("common.close")}
      actions={
        <>
          <BotaoDesktop variant="outlined" onClick={onCancel}>
            {t("common.cancel")}
          </BotaoDesktop>
          <BotaoDesktop size="sm" disabled={busy} onClick={onConfirm}>
            {t("settings.library.delete")}
          </BotaoDesktop>
        </>
      }
    >
      <p className={styles["text"]}>{shown?.criteria ? t("smart.delete_text") : t("settings.library.delete_text")}</p>
    </Dialogo>
  );
}
