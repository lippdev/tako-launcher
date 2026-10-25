import { useEffect, useId, useState } from "react";
import { CallError, call } from "../../bridge";
import type { GameEdit } from "../../bridge/generated/GameEdit";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeSelecao } from "../../components/CampoDeSelecao/CampoDeSelecao";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { editGameStrings as s } from "../strings";
import { draftOf, editProblem, isDirty, toSet, TRACKING_ORDER, type EditDraft, type EditProblem } from "./editarModel";
import styles from "./editar.module.css";

const TRACKING_OPTIONS = TRACKING_ORDER.map((value) => ({ value, label: s.trackingOptions[value] }));

interface DialogoEditarJogoProps {
  gameId: string;
  open: boolean;
  onClose: () => void;
  /** Gravou: o título já com os espaços tirados. */
  onSaved: (title: string) => void;
  /** Não foi possível nem ler os dados do jogo. */
  onError: (text: string) => void;
}

/**
 * "Editar jogo" (port do `edit_game` do Slint): nome, argumentos, pasta de trabalho e fim do jogo de um jogo
 * adicionado à mão. O arquivo que abre o jogo só aparece: trocar o arquivo é adicionar outro jogo.
 */
export function DialogoEditarJogo({ gameId, open, onClose, onSaved, onError }: DialogoEditarJogoProps) {
  const [edit, setEdit] = useState<GameEdit | null>(null);
  const [draft, setDraft] = useState<EditDraft | null>(null);
  const [saving, setSaving] = useState(false);
  const [problem, setProblem] = useState<EditProblem | null>(null);
  const formId = useId();

  // Cada abertura começa sem a sobra da anterior (ajuste de estado durante o desenho).
  const [wasOpen, setWasOpen] = useState(open);
  if (open !== wasOpen) {
    setWasOpen(open);
    if (open) {
      setEdit(null);
      setDraft(null);
      setProblem(null);
    }
  }

  // ... e lê os dados de novo.
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    call("game.edit", { gameId }).then(
      (loaded) => {
        if (cancelled) return;
        setEdit(loaded);
        setDraft(draftOf(loaded));
      },
      (failure: unknown) => {
        console.error("game.edit", failure);
        if (cancelled) return;
        onError(failure instanceof CallError && failure.error.code === "conflict" ? s.notManual : s.loadError);
        onClose();
      },
    );
    return () => {
      cancelled = true;
    };
  }, [open, gameId, onClose, onError]);

  if (!edit || !draft) return null;

  const change = (patch: Partial<EditDraft>) => {
    setDraft({ ...draft, ...patch });
    setProblem(null);
  };

  const save = async () => {
    if (saving) return;
    if (draft.title.trim() === "") {
      setProblem("name");
      return;
    }
    setSaving(true);
    try {
      const saved = await call("game.edit.set", toSet(gameId, draft));
      onSaved(saved.title);
    } catch (failure) {
      console.error("game.edit.set", failure);
      setProblem(editProblem(draft, failure instanceof CallError ? failure.error.code : "internal"));
    } finally {
      setSaving(false);
    }
  };

  const nameError = problem === "name" ? s.nameRequired : undefined;
  const folderError = problem === "folder" ? s.folderMissing(draft.workingDirectory.trim()) : undefined;
  const generalError =
    problem === "invalid" ? s.invalid : problem === "manual" ? s.notManual : problem === "save" ? s.saveError : undefined;

  return (
    <Dialogo
      open={open}
      onClose={onClose}
      title={s.title}
      closeLabel={s.close}
      size="m"
      actions={
        <>
          <BotaoDesktop variant="outlined" onClick={onClose}>
            {s.cancel}
          </BotaoDesktop>
          <BotaoDesktop type="submit" form={formId} disabled={saving || !isDirty(draft, edit)}>
            {s.save}
          </BotaoDesktop>
        </>
      }
    >
      <form
        id={formId}
        className={styles["form"]}
        onSubmit={(event) => {
          event.preventDefault();
          void save();
        }}
      >
        <CampoDeTexto
          label={s.name}
          value={draft.title}
          maxLength={200}
          autoFocus
          error={nameError}
          onChange={(title) => {
            change({ title });
          }}
        />
        <CampoDeTexto
          label={s.arguments}
          value={draft.arguments}
          maxLength={2048}
          placeholder={s.argumentsHint}
          onChange={(value) => {
            change({ arguments: value });
          }}
        />
        <CampoDeTexto
          label={s.folder}
          value={draft.workingDirectory}
          maxLength={1024}
          placeholder={edit.defaultFolder ?? ""}
          help={s.folderHint}
          error={folderError}
          onChange={(workingDirectory) => {
            change({ workingDirectory });
          }}
        />
        <GrupoDeConfiguracao>
          <LinhaDeConfiguracao
            title={s.tracking}
            control={
              <CampoDeSelecao
                label={s.tracking}
                options={TRACKING_OPTIONS}
                value={draft.tracking}
                onChange={(tracking) => {
                  change({ tracking });
                }}
              />
            }
          />
        </GrupoDeConfiguracao>
        <p className={styles["target"]}>
          <span className={styles["targetLabel"]}>{s.target}</span>
          <span className={styles["targetValue"]}>{edit.target}</span>
        </p>
        {generalError && (
          <p className={styles["problem"]} role="alert">
            {generalError}
          </p>
        )}
      </form>
    </Dialogo>
  );
}
