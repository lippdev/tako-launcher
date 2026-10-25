import { useEffect, useState, type FormEvent } from "react";
import { call } from "../../bridge";
import type { EmulatorsView } from "../../bridge/generated/EmulatorsView";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeSelecao } from "../../components/CampoDeSelecao/CampoDeSelecao";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { cx } from "../../lib/cx";
import { useLatest } from "../../lib/useLatest";
import { settingsStrings } from "../strings";
import type { Notify } from "./notice";
import styles from "./EmuladoresERoms.module.css";

const s = settingsStrings.emulators;

/** Valor de "uma subpasta por sistema" no seletor: o núcleo guarda `null`. */
const AUTO = "";

interface EmuladoresERomsProps {
  notify: Notify;
  /** "Ler de novo": a mesma leitura das lojas (`library.scan`). */
  onScan: () => void;
  scanning: boolean;
}

/** O que a linha de um emulador diz: onde ele abre, ou como achá-lo se faltar. */
function describeEmulator(emulator: EmulatorsView["emulators"][number]): string {
  if (!emulator.found) return s.missingDescription;
  if (emulator.chosen && emulator.path) return s.chosen(emulator.path);
  if (emulator.flatpak) return s.flatpak(emulator.flatpak);
  return emulator.path ?? "";
}

/**
 * Configurações › Biblioteca › Emuladores e ROMs (`settings_emulators` do Slint): as pastas de ROMs (adicionar,
 * escolher o sistema, remover), os emuladores achados neste PC e a leitura de novo. O núcleo lê as lojas de novo
 * sozinho a cada mudança de pasta; o andamento aparece na própria categoria.
 *
 * Escolher a pasta pela janela do sistema é coisa do hospedeiro; por enquanto o caminho vem digitado.
 */
export function EmuladoresERoms({ notify, onScan, scanning }: EmuladoresERomsProps) {
  const [view, setView] = useState<EmulatorsView | null>(null);
  const [busy, setBusy] = useState(false);
  const [path, setPath] = useState("");
  const [addError, setAddError] = useState<string | null>(null);
  const tell = useLatest(notify);

  useEffect(() => {
    let cancelled = false;
    call("emulators.list")
      .then((next) => {
        if (!cancelled) setView(next);
      })
      .catch((error: unknown) => {
        console.error("emulators.list falhou", error);
        if (!cancelled) tell.current("error", s.errors.load);
      });
    return () => {
      cancelled = true;
    };
  }, [tell]);

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
    change(() => call("emulators.folder.add", { path: typed }), s.added(typed), s.errors.add, {
      onDone: () => {
        setPath("");
        setAddError(null);
      },
      // O núcleo não aceitou o caminho: o erro fica no campo, onde o usuário corrige.
      onError: () => {
        setAddError(s.errors.add);
      },
    });
  }

  const systemOptions = [{ value: AUTO, label: s.folderAuto }, ...(view?.systems ?? []).map((system) => ({ value: system.id, label: system.name }))];

  return (
    <>
      <p className={styles["intro"]}>{s.text}</p>
      <GrupoDeConfiguracao title={s.folders}>
        {view && view.romFolders.length === 0 && <LinhaDeConfiguracao title={s.noFolders} />}
        {view?.romFolders.map((folder) => (
          <LinhaDeConfiguracao
            key={folder.path}
            title={folder.path}
            control={
              <span className={styles["actions"]}>
                <CampoDeSelecao
                  label={s.folderSystem(folder.path)}
                  options={systemOptions}
                  value={folder.system ?? AUTO}
                  disabled={busy}
                  onChange={(system) => {
                    change(
                      () => call("emulators.folder.system.set", { path: folder.path, system: system === AUTO ? null : system }),
                      null,
                      s.errors.system,
                    );
                  }}
                />
                <BotaoDesktop
                  variant="outlined"
                  aria-label={s.removeFor(folder.path)}
                  disabled={busy}
                  onClick={() => {
                    change(() => call("emulators.folder.remove", { path: folder.path }), s.removed, s.errors.remove);
                  }}
                >
                  {s.remove}
                </BotaoDesktop>
              </span>
            }
          />
        ))}
        <LinhaDeConfiguracao title={s.add.title} description={s.add.description} />
        <form className={styles["add"]} onSubmit={add}>
          <CampoDeTexto
            className={cx(styles["addField"])}
            label={s.add.label}
            placeholder={s.add.placeholder}
            value={path}
            help={s.add.help}
            error={addError ?? undefined}
            spellCheck={false}
            autoComplete="off"
            onChange={(next) => {
              setPath(next);
              setAddError(null);
            }}
          />
          <BotaoDesktop variant="outlined" type="submit" disabled={busy || path.trim() === ""} className={styles["addButton"]}>
            {s.add.action}
          </BotaoDesktop>
        </form>
        <LinhaDeConfiguracao
          title={s.rescan.title}
          description={s.rescan.description}
          control={
            <BotaoDesktop variant="outlined" icon="refresh" disabled={scanning} onClick={onScan}>
              {s.rescan.action}
            </BotaoDesktop>
          }
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={s.emulators}>
        {view?.emulators.map((emulator) => (
          <LinhaDeConfiguracao
            key={emulator.key}
            title={emulator.name}
            description={describeEmulator(emulator)}
            control={<span className={cx(styles["status"], emulator.found && styles["found"])}>{emulator.found ? s.found : s.missing}</span>}
          />
        ))}
      </GrupoDeConfiguracao>
    </>
  );
}
