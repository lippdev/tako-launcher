import { useState, type FormEvent } from "react";
import { call } from "../../bridge";
import type { Collection } from "../../bridge/generated/Collection";
import type { SmartCriteria } from "../../bridge/generated/SmartCriteria";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { useT } from "../../i18n";
import { collectionStrings as s } from "../strings";
import { cleanName, nameProblem, ofKind } from "./colecoesModel";
import { problemText } from "./problemaDoNome";
import styles from "./Colecoes.module.css";

interface SalvarColecaoProps {
  open: boolean;
  onClose: () => void;
  collections: readonly Collection[];
  /** A regra da coleção: os filtros e a busca que a lista mostra agora. */
  rule: SmartCriteria;
  /** A coleção foi salva (`false`) ou o núcleo recusou (`true`). */
  onDone: (error: boolean, message: string) => void;
}

/** "Salvar como coleção automática": pede o nome e guarda os filtros atuais (`collections.smart.set`). */
export function SalvarColecao({ open, onClose, collections, rule, onDone }: SalvarColecaoProps) {
  const t = useT();
  const [name, setName] = useState("");
  const [touched, setTouched] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const problem = nameProblem(name, ofKind(collections, true));
  const error = failed ? s.saveError : touched && problem ? problemText(t, problem) : undefined;

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (busy) return;
    setTouched(true);
    if (problem) return;
    setBusy(true);
    setFailed(false);
    void call("collections.smart.set", { collectionId: null, name: cleanName(name), criteria: rule })
      .then(() => {
        onDone(false, t("smart.saved"));
        onClose();
      })
      .catch((e: unknown) => {
        console.error("coleções: salvar", e);
        setFailed(true);
      })
      .finally(() => {
        setBusy(false);
      });
  };

  return (
    <Dialogo
      open={open}
      onClose={onClose}
      size="s"
      title={t("smart.save")}
      closeLabel={t("common.close")}
      actions={
        <>
          <BotaoDesktop variant="outlined" type="button" onClick={onClose}>
            {t("common.cancel")}
          </BotaoDesktop>
          <BotaoDesktop size="sm" type="submit" form="colecao-salvar" disabled={busy}>
            {t("common.save")}
          </BotaoDesktop>
        </>
      }
    >
      <form id="colecao-salvar" className={styles["form"]} onSubmit={submit}>
        <p className={styles["text"]}>{t("smart.save_desc")}</p>
        <CampoDeTexto
          label={s.nameLabel}
          value={name}
          help={t("smart.name_rule")}
          error={error}
          onChange={(value) => {
            setName(value);
            setTouched(true);
            setFailed(false);
          }}
        />
      </form>
    </Dialogo>
  );
}
