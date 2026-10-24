import { useState, type FormEvent } from "react";
import { call } from "../../bridge";
import type { Collection } from "../../bridge/generated/Collection";
import type { SmartCriteria } from "../../bridge/generated/SmartCriteria";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { cx } from "../../lib/cx";
import { useT } from "../../i18n";
import { collectionStrings as s } from "../strings";
import { cleanName, nameProblem, ofKind } from "./colecoesModel";
import { problemText } from "./problemaDoNome";
import styles from "./Colecoes.module.css";

interface OrganizarColecoesProps {
  open: boolean;
  onClose: () => void;
  collections: readonly Collection[];
  /** A regra que a lista mostra agora; "Usar os filtros atuais" troca a da coleção automática por ela. */
  rule: SmartCriteria;
  /** Uma coleção foi apagada (a lista sai dela se estava aberta). */
  onDeleted: (id: string) => void;
  /** A ação deu certo (`false`) ou o núcleo recusou (`true`). */
  onDone: (error: boolean, message: string) => void;
}

interface Renaming {
  id: string;
  name: string;
  auto: boolean;
}

/**
 * Organizar coleções: criar uma manual, e renomear, apagar ou (nas automáticas) trocar os filtros pelos da lista.
 * Manuais e automáticas aparecem juntas, como no seletor da lista.
 */
export function OrganizarColecoes({ open, onClose, collections, rule, onDeleted, onDone }: OrganizarColecoesProps) {
  const t = useT();
  const [busy, setBusy] = useState(false);
  const [newName, setNewName] = useState("");
  const [newTouched, setNewTouched] = useState(false);
  const [renaming, setRenaming] = useState<Renaming | null>(null);
  const [deleting, setDeleting] = useState<Collection | null>(null);
  // O diálogo de exclusão sai com animação; o nome fica até ele sumir.
  const [lastDeleting, setLastDeleting] = useState<Collection | null>(null);

  const run = (action: () => Promise<unknown>, message: string, after?: () => void) => {
    if (busy) return;
    setBusy(true);
    void action()
      .then(() => {
        onDone(false, message);
        after?.();
      })
      .catch((e: unknown) => {
        console.error("coleções: ação", e);
        onDone(true, s.saveError);
      })
      .finally(() => {
        setBusy(false);
      });
  };

  const newProblem = nameProblem(newName, ofKind(collections, false));
  const create = (event: FormEvent) => {
    event.preventDefault();
    setNewTouched(true);
    if (newProblem) return;
    run(
      () => call("collections.create", { name: cleanName(newName) }),
      s.created,
      () => {
        setNewName("");
        setNewTouched(false);
      },
    );
  };

  const renameProblem = renaming ? nameProblem(renaming.name, ofKind(collections, renaming.auto), renaming.id) : null;
  const rename = (event: FormEvent) => {
    event.preventDefault();
    if (!renaming || renameProblem) return;
    const { id, name } = renaming;
    run(
      () => call("collections.rename", { collectionId: id, name: cleanName(name) }),
      s.renamed,
      () => {
        setRenaming(null);
      },
    );
  };

  const closeDelete = () => {
    setDeleting(null);
  };
  const confirmDelete = () => {
    if (!deleting) return;
    const { id } = deleting;
    run(
      () => call("collections.delete", { collectionId: id }),
      s.deleted,
      () => {
        onDeleted(id);
        closeDelete();
      },
    );
  };

  const shownDeleting = deleting ?? lastDeleting;

  return (
    <>
      <Dialogo
        open={open}
        onClose={onClose}
        size="m"
        title={t("desktop.collections_manage")}
        closeLabel={t("common.close")}
      >
        <form className={styles["create"]} onSubmit={create}>
          <CampoDeTexto
            className={cx(styles["createField"])}
            label={t("collections.new")}
            value={newName}
            help={t("collections.name_rule")}
            error={newTouched && newProblem ? problemText(t, newProblem) : undefined}
            onChange={(value) => {
              setNewName(value);
              setNewTouched(true);
            }}
          />
          <BotaoDesktop variant="outlined" type="submit" disabled={busy} className={styles["createButton"]}>
            {t("settings.library.collections_create")}
          </BotaoDesktop>
        </form>
        {collections.length === 0 ? (
          <p className={styles["empty"]}>{t("desktop.no_collections")}</p>
        ) : (
          <ul className={styles["list"]} aria-label={t("collections.title")}>
            {collections.map((c) =>
              renaming?.id === c.id ? (
                <li key={c.id} className={styles["row"]}>
                  <form className={styles["renameForm"]} onSubmit={rename}>
                    <CampoDeTexto
                      className={cx(styles["createField"])}
                      label={s.renameFor(c.name)}
                      hideLabel
                      value={renaming.name}
                      error={renameProblem ? problemText(t, renameProblem) : undefined}
                      onChange={(name) => {
                        setRenaming({ ...renaming, name });
                      }}
                    />
                    <BotaoDesktop variant="outlined" type="submit" disabled={busy}>
                      {t("common.save")}
                    </BotaoDesktop>
                    <BotaoDesktop
                      variant="outlined"
                      onClick={() => {
                        setRenaming(null);
                      }}
                    >
                      {t("common.cancel")}
                    </BotaoDesktop>
                  </form>
                </li>
              ) : (
                <li key={c.id} className={styles["row"]}>
                  <span className={styles["name"]}>{c.name}</span>
                  <span className={styles["kind"]}>
                    {c.criteria ? t("desktop.collection_auto") : t("common.games", { n: c.gameIds.length })}
                  </span>
                  <span className={styles["rowActions"]}>
                    <BotaoDesktop
                      variant="outlined"
                      aria-label={s.renameFor(c.name)}
                      disabled={busy}
                      onClick={() => {
                        setRenaming({ id: c.id, name: c.name, auto: c.criteria !== null });
                      }}
                    >
                      {t("settings.library.rename")}
                    </BotaoDesktop>
                    {c.criteria && (
                      <BotaoDesktop
                        variant="outlined"
                        aria-label={s.useCurrentFor(c.name)}
                        disabled={busy}
                        onClick={() => {
                          run(
                            () => call("collections.smart.set", { collectionId: c.id, name: c.name, criteria: rule }),
                            s.savedRule,
                          );
                        }}
                      >
                        {s.useCurrent}
                      </BotaoDesktop>
                    )}
                    <BotaoDesktop
                      variant="outlined"
                      aria-label={s.deleteFor(c.name)}
                      disabled={busy}
                      onClick={() => {
                        setLastDeleting(c);
                        setDeleting(c);
                      }}
                    >
                      {t("settings.library.delete")}
                    </BotaoDesktop>
                  </span>
                </li>
              ),
            )}
          </ul>
        )}
      </Dialogo>

      <Dialogo
        open={deleting !== null}
        onClose={closeDelete}
        size="s"
        title={t("settings.library.delete_title", { name: shownDeleting?.name ?? "" })}
        closeLabel={t("common.close")}
        actions={
          <>
            <BotaoDesktop variant="outlined" onClick={closeDelete}>
              {t("common.cancel")}
            </BotaoDesktop>
            <BotaoDesktop size="sm" disabled={busy} onClick={confirmDelete}>
              {t("settings.library.delete")}
            </BotaoDesktop>
          </>
        }
      >
        <p className={styles["text"]}>
          {shownDeleting?.criteria ? t("smart.delete_text") : t("settings.library.delete_text")}
        </p>
      </Dialogo>
    </>
  );
}
