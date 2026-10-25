import { useEffect, useRef, useState, type FormEvent } from "react";
import { call, on } from "../../bridge";
import type { EmulatorsView } from "../../bridge/generated/EmulatorsView";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeSelecao } from "../../components/CampoDeSelecao/CampoDeSelecao";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT, type TFunction } from "../../i18n";
import { cx } from "../../lib/cx";
import { useLatest } from "../../lib/useLatest";
import type { Notify } from "./notice";
import styles from "./EmuladoresERoms.module.css";

/** Valor de "uma subpasta por sistema" no seletor: o núcleo guarda `null`. */
const AUTO = "";

interface EmuladoresERomsProps {
  notify: Notify;
  /** "Ler de novo": a mesma leitura das lojas (`library.scan`). */
  onScan: () => void;
  scanning: boolean;
}

/** O que a linha de um emulador diz: onde ele abre, ou como achá-lo se faltar. */
function describeEmulator(emulator: EmulatorsView["emulators"][number], t: TFunction): string {
  if (!emulator.found) return t("settings.emulators.missing_desc");
  if (emulator.chosen && emulator.path) return t("settings.emulators.chosen_desc", { path: emulator.path });
  if (emulator.flatpak) return t("settings.emulators_web.flatpak", { id: emulator.flatpak });
  return emulator.path ?? "";
}

/**
 * Configurações › Biblioteca › Emuladores e ROMs (`settings_emulators` do Slint): as pastas de ROMs (adicionar,
 * escolher o sistema, remover), os emuladores achados neste PC e a leitura de novo. O núcleo lê as lojas de novo
 * sozinho a cada mudança de pasta; o andamento aparece na própria categoria.
 */
export function EmuladoresERoms({ notify, onScan, scanning }: EmuladoresERomsProps) {
  const t = useT();
  const [view, setView] = useState<EmulatorsView | null>(null);
  const [busy, setBusy] = useState(false);
  const [path, setPath] = useState("");
  const [addError, setAddError] = useState<string | null>(null);
  const pendingPickId = useRef<number | null>(null);
  const tell = useLatest(notify);

  useEffect(() => {
    let cancelled = false;
    call("emulators.list")
      .then((next) => {
        if (!cancelled) setView(next);
      })
      .catch((error: unknown) => {
        console.error("emulators.list falhou", error);
        if (!cancelled) tell.current("error", t("settings.emulators_web.err_load"));
      });
    return () => {
      cancelled = true;
    };
  }, [tell, t]);

  useEffect(() => {
    return on("system.picked", ({ id, path: pickedPath }) => {
      if (pendingPickId.current === id) {
        pendingPickId.current = null;
        if (pickedPath !== null) {
          setPath(pickedPath);
          setAddError(null);
        }
      }
    });
  }, []);

  const pickFolder = async () => {
    try {
      const res = await call("system.pick", {
        kind: "folder",
        purpose: "settings.emulators.add_folder",
      });
      pendingPickId.current = res.id;
    } catch (failure) {
      console.error("system.pick", failure);
    }
  };

  /** Roda uma chamada que devolve a lista nova. Se falhar, `onError` decide o que mostrar (senão, o aviso). */
  function change(action: () => Promise<EmulatorsView>, done: string | null, failure: string, options?: { onDone?: () => void; onError?: () => void }) {
    if (busy) return;
    setBusy(true);
    action()
      .then((next) => {
        setView(next);
        if (done) notify("success", done);
        options?.onDone?.();
      })
      .catch((error: unknown) => {
        console.error("emuladores: a chamada falhou", error);
        if (options?.onError) options.onError();
        else notify("error", failure);
      })
      .finally(() => {
        setBusy(false);
      });
  }

  function add(event: FormEvent) {
    event.preventDefault();
    const typed = path.trim();
    if (!typed) return;
    change(() => call("emulators.folder.add", { path: typed }), t("settings.emulators.folder_added", { path: typed }), t("settings.emulators_web.err_add"), {
      onDone: () => {
        setPath("");
        setAddError(null);
      },
      // O núcleo não aceitou o caminho: o erro fica no campo, onde o usuário corrige.
      onError: () => {
        setAddError(t("settings.emulators_web.err_add"));
      },
    });
  }

  const systemOptions = [{ value: AUTO, label: t("settings.emulators.folder_auto") }, ...(view?.systems ?? []).map((system) => ({ value: system.id, label: system.name }))];

  return (
    <>
      <p className={styles["intro"]}>{t("settings.emulators.text")}</p>
      <GrupoDeConfiguracao title={t("settings.emulators.folders_header")}>
        {view && view.romFolders.length === 0 && <LinhaDeConfiguracao title={t("settings.emulators_web.no_folders")} />}
        {view?.romFolders.map((folder) => (
          <LinhaDeConfiguracao
            key={folder.path}
            title={folder.path}
            control={
              <span className={styles["actions"]}>
                <CampoDeSelecao
                  label={t("settings.emulators_web.folder_system", { path: folder.path })}
                  options={systemOptions}
                  value={folder.system ?? AUTO}
                  disabled={busy}
                  onChange={(system) => {
                    change(
                      () => call("emulators.folder.system.set", { path: folder.path, system: system === AUTO ? null : system }),
                      null,
                      t("settings.emulators_web.err_system"),
                    );
                  }}
                />
                <BotaoDesktop
                  variant="outlined"
                  aria-label={t("settings.emulators_web.remove_for", { path: folder.path })}
                  disabled={busy}
                  onClick={() => {
                    change(() => call("emulators.folder.remove", { path: folder.path }), t("settings.emulators.folder_removed"), t("settings.emulators_web.err_remove"));
                  }}
                >
                  {t("settings.emulators.remove_folder")}
                </BotaoDesktop>
              </span>
            }
          />
        ))}
        <LinhaDeConfiguracao title={t("settings.emulators.add_folder")} description={t("settings.emulators.add_folder_desc")} />
        <form className={styles["add"]} onSubmit={add}>
          <CampoDeTexto
            className={cx(styles["addField"])}
            label={t("settings.emulators_web.add_label")}
            placeholder={t("settings.emulators_web.add_placeholder")}
            value={path}
            help={t("settings.emulators_web.add_help")}
            error={addError ?? undefined}
            spellCheck={false}
            autoComplete="off"
            onChange={(next) => {
              setPath(next);
              setAddError(null);
            }}
          />
          <BotaoDesktop
            variant="outlined"
            type="button"
            onClick={() => {
              void pickFolder();
            }}
            className={styles["addButton"]}
          >
            {t("settings.emulators_web.add_choose")}
          </BotaoDesktop>
          <BotaoDesktop variant="outlined" type="submit" disabled={busy || path.trim() === ""} className={styles["addButton"]}>
            {t("settings.emulators_web.add_action")}
          </BotaoDesktop>
        </form>
        <LinhaDeConfiguracao
          title={t("settings.emulators_web.rescan_title")}
          description={t("settings.emulators_web.rescan_desc")}
          control={
            <BotaoDesktop variant="outlined" icon="refresh" disabled={scanning} onClick={onScan}>
              {t("settings.emulators_web.rescan_action")}
            </BotaoDesktop>
          }
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={t("settings.emulators.emulators_header")}>
        {view?.emulators.map((emulator) => (
          <LinhaDeConfiguracao
            key={emulator.key}
            title={emulator.name}
            description={describeEmulator(emulator, t)}
            control={<span className={cx(styles["status"], emulator.found && styles["found"])}>{emulator.found ? t("settings.emulators.found") : t("settings.emulators.missing")}</span>}
          />
        ))}
      </GrupoDeConfiguracao>
    </>
  );
}
