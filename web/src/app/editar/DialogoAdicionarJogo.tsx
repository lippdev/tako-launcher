import { useEffect, useId, useRef, useState } from "react";
import { CallError, call, on } from "../../bridge";
import type { RomSystem } from "../../bridge/generated/RomSystem";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { addGameStrings as s } from "../strings";
import { addOutcome, addProblem, type AddProblem } from "./editarModel";
import styles from "./editar.module.css";

/** Depois do caminho, uma ROM pergunta o sistema (os prováveis, e "Outro sistema" mostra os demais). */
interface SystemStep {
  fileName: string;
  systems: readonly RomSystem[];
  others: readonly RomSystem[];
  showOthers: boolean;
}

const PROBLEM_TEXT: Record<AddProblem, string> = {
  empty: s.empty,
  invalid: s.invalid,
  "not-found": s.notFound,
  error: s.error,
};

interface DialogoAdicionarJogoProps {
  open: boolean;
  onClose: () => void;
  /** O jogo entrou na biblioteca; o texto já vem pronto ("X adicionado"). */
  onAdded: (text: string) => void;
}

/**
 * "Adicionar jogo" (port do `add_game` do Slint): o arquivo que abre o jogo (executável, atalho ou ROM). O título sai
 * do nome do produto do `.exe` ou do nome do arquivo; ROM pergunta de qual sistema é. O nome pode ser mudado depois
 * em Editar.
 */
export function DialogoAdicionarJogo({ open, onClose, onAdded }: DialogoAdicionarJogoProps) {
  const [path, setPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<AddProblem | null>(null);
  const [already, setAlready] = useState<string | null>(null);
  const [unreadable, setUnreadable] = useState(false);
  const [step, setStep] = useState<SystemStep | null>(null);
  const pendingPickId = useRef<number | null>(null);
  const formId = useId();

  // Cada abertura começa em branco (ajuste de estado durante o desenho).
  const [wasOpen, setWasOpen] = useState(open);
  if (open !== wasOpen) {
    setWasOpen(open);
    if (open) {
      setPath("");
      setBusy(false);
      setProblem(null);
      setAlready(null);
      setUnreadable(false);
      setStep(null);
    }
  }

  useEffect(() => {
    pendingPickId.current = null;
  }, [open]);

  useEffect(() => {
    return on("system.picked", ({ id, path: pickedPath }) => {
      if (pendingPickId.current === id) {
        pendingPickId.current = null;
        if (pickedPath !== null) {
          setPath(pickedPath);
          setProblem(null);
          setAlready(null);
          setUnreadable(false);
        }
      }
    });
  }, []);

  const pickFile = async () => {
    try {
      const res = await call("system.pick", {
        kind: "file",
        purpose: "add_game",
      });
      pendingPickId.current = res.id;
    } catch (failure) {
      console.error("system.pick", failure);
    }
  };

  const submit = async (system?: RomSystem) => {
    if (busy) return;
    if (path.trim() === "") {
      setProblem("empty");
      return;
    }
    setBusy(true);
    setProblem(null);
    setAlready(null);
    setUnreadable(false);
    try {
      const outcome = addOutcome(await call("library.add_game", { path, system: system?.key ?? null }));
      switch (outcome.kind) {
        case "added":
          onAdded(system ? s.addedWithSystem(outcome.title, system.name) : s.added(outcome.title));
          break;
        case "ask-system":
          setStep({ fileName: outcome.fileName, systems: outcome.systems, others: outcome.others, showOthers: false });
          break;
        case "already":
          setStep(null);
          setAlready(outcome.title);
          break;
        case "unreadable":
          setStep(null);
          setUnreadable(true);
          break;
      }
    } catch (failure) {
      console.error("library.add_game", failure);
      setStep(null);
      setProblem(addProblem(path, failure instanceof CallError ? failure.error.code : "internal"));
    } finally {
      setBusy(false);
    }
  };

  const systems = step ? (step.showOthers ? [...step.systems, ...step.others] : step.systems) : [];

  return (
    <Dialogo
      open={open}
      onClose={onClose}
      title={step ? s.whichSystem : s.title}
      closeLabel={s.close}
      size="m"
      actions={
        step ? (
          <BotaoDesktop variant="outlined" onClick={onClose}>
            {s.cancel}
          </BotaoDesktop>
        ) : (
          <>
            <BotaoDesktop variant="outlined" onClick={onClose}>
              {s.cancel}
            </BotaoDesktop>
            <BotaoDesktop type="submit" form={formId} disabled={busy || path.trim() === ""}>
              {busy ? s.adding : s.add}
            </BotaoDesktop>
          </>
        )
      }
    >
      {step ? (
        <div className={styles["form"]}>
          <p className={styles["lead"]}>
            <strong>{step.fileName}</strong>
            <br />
            {s.systemText}
          </p>
          <GrupoDeConfiguracao>
            {systems.map((system) => (
              <LinhaDeConfiguracao
                key={system.key}
                title={system.name}
                onClick={() => {
                  void submit(system);
                }}
              />
            ))}
            {!step.showOthers && step.others.length > 0 && (
              <LinhaDeConfiguracao
                title={s.otherSystem}
                onClick={() => {
                  setStep({ ...step, showOthers: true });
                }}
              />
            )}
          </GrupoDeConfiguracao>
        </div>
      ) : (
        <form
          id={formId}
          className={styles["form"]}
          onSubmit={(event) => {
            event.preventDefault();
            void submit();
          }}
        >
          <div className={styles["typingRow"]}>
            <CampoDeTexto
              label={s.file}
              value={path}
              maxLength={1024}
              placeholder={s.filePlaceholder}
              help={s.fileHint}
              autoFocus
              error={problem ? PROBLEM_TEXT[problem] : unreadable ? s.unreadable : undefined}
              onChange={(value) => {
                setPath(value);
                setProblem(null);
                setAlready(null);
                setUnreadable(false);
              }}
            />
            <BotaoDesktop
              type="button"
              variant="outlined"
              onClick={() => {
                void pickFile();
              }}
            >
              {s.choose}
            </BotaoDesktop>
          </div>
          {already !== null && (
            <p className={styles["info"]} role="status">
              {s.already(already)}
            </p>
          )}
        </form>
      )}
    </Dialogo>
  );
}
