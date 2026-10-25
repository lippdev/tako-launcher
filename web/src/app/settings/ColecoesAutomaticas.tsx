import { useCallback, useEffect, useState, type FormEvent } from "react";
import { call, on } from "../../bridge";
import type { Collection } from "../../bridge/generated/Collection";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { useLatest } from "../../lib/useLatest";
import { cleanName, nameProblem, ofKind } from "../colecoes/colecoesModel";
import { ExcluirColecao } from "../colecoes/ExcluirColecao";
import { problemText } from "../colecoes/problemaDoNome";
import { settingsStrings } from "../strings";
import type { Notify } from "./notice";
import styles from "./ColecoesAutomaticas.module.css";

const s = settingsStrings.library.smart;

interface AutoCollection {
  collection: Collection;
  /** Quantos jogos a coleção mostra agora; `null` enquanto não chegou (ou se a consulta falhou). */
  games: number | null;
}

/**
 * As coleções automáticas e quantos jogos cada uma mostra agora. O núcleo resolve os jogos na consulta, então a conta
 * vem de uma página de um jogo em `library.list`. Refaz a lista quando a biblioteca muda.
 */
function useAutoCollections(onError: (text: string) => void) {
  const [items, setItems] = useState<AutoCollection[] | null>(null);
  const [version, setVersion] = useState(0);
  // Quem chama passa uma função nova a cada desenho; ler a mais recente evita refazer a consulta a cada desenho.
  const fail = useLatest(onError);

  useEffect(() => {
    let cancelled = false;
    const load = () => {
      void (async () => {
        try {
          const autos = ofKind(await call("collections.list"), true);
          const counted = await Promise.all(
            autos.map(async (collection): Promise<AutoCollection> => {
              try {
                const page = await call("library.list", { collection: collection.id, limit: 1 });
                return { collection, games: page.total };
              } catch (error) {
                console.error("library.list falhou", error);
                return { collection, games: null };
              }
            }),
          );
          if (!cancelled) setItems(counted);
        } catch (error) {
          console.error("collections.list falhou", error);
          if (!cancelled) fail.current(s.loadError);
        }
      })();
    };
    load();
    const stop = on("library.changed", load);
    return () => {
      cancelled = true;
      stop();
    };
  }, [version, fail]);

  return { items, reload: useCallback(() => { setVersion((v) => v + 1); }, []) };
}

interface ColecoesAutomaticasProps {
  notify: Notify;
}

/**
 * Configurações › Biblioteca › Coleções automáticas (`smart::panel` do Slint): cada uma com quantos jogos mostra, e
 * renomear e excluir. O nome segue as regras de sempre (`colecoes/`); excluir pergunta antes e não toca nos jogos.
 */
export function ColecoesAutomaticas({ notify }: ColecoesAutomaticasProps) {
  const t = useT();
  const { items, reload } = useAutoCollections((text) => { notify("error", text); });
  const [renaming, setRenaming] = useState<{ id: string; name: string } | null>(null);
  const [deleting, setDeleting] = useState<Collection | null>(null);
  const [busy, setBusy] = useState(false);
  // O diálogo de renomear sai com animação; o nome da linha fica até ele sumir.
  const [renameOpen, setRenameOpen] = useState(false);

  const all = items?.map((item) => item.collection) ?? [];
  const problem = renaming ? nameProblem(renaming.name, all, renaming.id) : null;

  function run(action: () => Promise<unknown>, message: string, after: () => void) {
    if (busy) return;
    setBusy(true);
    void action()
      .then(() => {
        notify("success", message);
        after();
        reload();
      })
      .catch((error: unknown) => {
        console.error("coleções automáticas: ação falhou", error);
        notify("error", s.saveError);
      })
      .finally(() => {
        setBusy(false);
      });
  }

  function rename(event: FormEvent) {
    event.preventDefault();
    if (!renaming || problem) return;
    const { id, name } = renaming;
    run(() => call("collections.rename", { collectionId: id, name: cleanName(name) }), s.renamed, () => {
      setRenameOpen(false);
    });
  }

  function confirmDelete() {
    if (!deleting) return;
    const { id } = deleting;
    run(() => call("collections.delete", { collectionId: id }), s.deleted, () => {
      setDeleting(null);
    });
  }

  return (
    <>
      <p className={styles["intro"]}>{s.text}</p>
      <GrupoDeConfiguracao title={s.section}>
        {items && items.length === 0 && <LinhaDeConfiguracao title={s.empty} />}
        {items?.map(({ collection, games }) => (
          <LinhaDeConfiguracao
            key={collection.id}
            title={collection.name}
            {...(games !== null ? { description: s.count(games) } : {})}
            control={
              <>
                <BotaoDesktop
                  variant="outlined"
                  aria-label={s.rename(collection.name)}
                  disabled={busy}
                  onClick={() => {
                    setRenaming({ id: collection.id, name: collection.name });
                    setRenameOpen(true);
                  }}
                >
                  {t("settings.library.rename")}
                </BotaoDesktop>
                <BotaoDesktop
                  variant="outlined"
                  aria-label={s.remove(collection.name)}
                  disabled={busy}
                  onClick={() => {
                    setDeleting(collection);
                  }}
                >
                  {t("settings.library.delete")}
                </BotaoDesktop>
              </>
            }
          />
        ))}
      </GrupoDeConfiguracao>

      <Dialogo
        open={renameOpen}
        onClose={() => {
          setRenameOpen(false);
        }}
        size="s"
        title={s.renameTitle}
        closeLabel={t("common.close")}
        actions={
          <>
            <BotaoDesktop
              variant="outlined"
              onClick={() => {
                setRenameOpen(false);
              }}
            >
              {t("common.cancel")}
            </BotaoDesktop>
            <BotaoDesktop size="sm" type="submit" form="renomear-automatica" disabled={busy || problem !== null}>
              {t("common.save")}
            </BotaoDesktop>
          </>
        }
      >
        <form id="renomear-automatica" onSubmit={rename}>
          <CampoDeTexto
            label={s.nameLabel}
            value={renaming?.name ?? ""}
            help={t("smart.name_rule")}
            error={problem ? problemText(t, problem) : undefined}
            onChange={(name) => {
              setRenaming((previous) => (previous ? { ...previous, name } : previous));
            }}
          />
        </form>
      </Dialogo>

      <ExcluirColecao
        collection={deleting}
        busy={busy}
        onCancel={() => {
          setDeleting(null);
        }}
        onConfirm={confirmDelete}
      />
    </>
  );
}
