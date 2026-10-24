import { useState, type FormEvent } from "react";
import { CallError, call } from "../../bridge";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { useT } from "../../i18n";
import { profileStrings as s } from "../strings";

/** Maior nome de coleção que o núcleo aceita (`collections.name_rule`: "Até 40 caracteres"). */
const MAX_NAME = 40;

interface NovaColecaoDialogoProps {
  open: boolean;
  onClose: () => void;
}

/**
 * "Nova coleção" da guia Coleções: pede o nome e cria pelo núcleo (`collections.create`). A coleção nova chega à
 * tela pelo `library.changed` que o núcleo emite.
 */
export function NovaColecaoDialogo({ open, onClose }: NovaColecaoDialogoProps) {
  const t = useT();
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Cada abertura começa com o campo vazio.
  const [wasOpen, setWasOpen] = useState(false);
  if (open !== wasOpen) {
    setWasOpen(open);
    if (open) {
      setName("");
      setError(null);
    }
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    try {
      await call("collections.create", { name });
      onClose();
    } catch (failure) {
      // Nome vazio, repetido ou grande demais volta como `bad_request`; o resto é falha do núcleo.
      const invalid = failure instanceof CallError && failure.error.code === "bad_request";
      if (!invalid) console.error("collections.create", failure);
      setError(invalid ? t("collections.name_rule") : s.saveError);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialogo
      open={open}
      onClose={onClose}
      size="m"
      closeLabel={t("common.close")}
      title={t("collections.new")}
      actions={
        <>
          <BotaoDesktop variant="secondary" size="sm" onClick={onClose}>
            {t("common.cancel")}
          </BotaoDesktop>
          <BotaoDesktop variant="primary" size="sm" type="submit" form="colecao-nome" disabled={busy || name.trim() === ""}>
            {t("common.save")}
          </BotaoDesktop>
        </>
      }
    >
      <form id="colecao-nome" onSubmit={(event) => void submit(event)}>
        <CampoDeTexto
          label={t("collections.name_rule")}
          value={name}
          onChange={(value) => {
            setName(value);
            setError(null);
          }}
          maxLength={MAX_NAME}
          autoFocus
          autoComplete="off"
          {...(error ? { error } : {})}
        />
      </form>
    </Dialogo>
  );
}
