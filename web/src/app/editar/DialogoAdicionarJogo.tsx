import { useEffect, useId, useMemo, useRef, useState } from "react";
import { CallError, call, on } from "../../bridge";
import type { RomSystem } from "../../bridge/generated/RomSystem";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { addOutcome, addProblem, type AddProblem } from "./editarModel";
import styles from "./editar.module.css";

/** Depois do caminho, uma ROM pergunta o sistema (os prováveis, e "Outro sistema" mostra os demais). */
interface SystemStep {
  fileName: string;
  systems: readonly RomSystem[];
  others: readonly RomSystem[];
  showOthers: boolean;
}

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
  const t = useT();
  const [path, setPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<AddProblem | null>(null);
  const [already, setAlready] = useState<string | null>(null);
  const [unreadable, setUnreadable] = useState(false);
  const [step, setStep] = useState<SystemStep | null>(null);
  const pendingPickId = useRef<number | null>(null);
  const formId = useId();

  const problemText: Record<AddProblem, string> = useMemo(
    () => ({
      empty: t("add_game.empty"),
      invalid: t("add_game.invalid"),
      "not-found": t("add_game.not_found"),
      error: t("add_game.error"),
    }),
    [t],
  );

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
          onAdded(system ? t("add_game.added_with_system", { game: outcome.title, system: system.name }) : t("add_game.added", { game: outcome.title }));
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
      title={step ? t("add_game.which_system") : t("add_game.title")}
      closeLabel={t("common.close")}
      size="m"
      actions={
        step ? (
          <BotaoDesktop variant="outlined" onClick={onClose}>
            {t("common.cancel")}
          </BotaoDesktop>
        ) : (
          <>
            <BotaoDesktop variant="outlined" onClick={onClose}>
              {t("common.cancel")}
            </BotaoDesktop>
            <BotaoDesktop type="submit" form={formId} disabled={busy || path.trim() === ""}>
              {busy ? t("add_game.adding") : t("add_game.add")}
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
            {t("add_game.system_text")}
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
                title={t("add_game.other_system")}
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
              label={t("add_game.file")}
              value={path}
              maxLength={1024}
              placeholder={t("add_game.file_placeholder")}
              help={t("add_game.file_hint")}
              autoFocus
              error={problem ? problemText[problem] : unreadable ? t("add_game.unreadable") : undefined}
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
              {t("add_game.choose")}
            </BotaoDesktop>
          </div>
          {already !== null && (
            <p className={styles["info"]} role="status">
              {t("add_game.already", { game: already })}
            </p>
          )}
        </form>
      )}
    </Dialogo>
  );
}
