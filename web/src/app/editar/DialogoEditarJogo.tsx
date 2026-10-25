import { useEffect, useId, useMemo, useRef, useState } from "react";
import { CallError, call, on } from "../../bridge";
import type { GameEdit } from "../../bridge/generated/GameEdit";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeSelecao } from "../../components/CampoDeSelecao/CampoDeSelecao";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { draftOf, editProblem, isDirty, toSet, TRACKING_ORDER, type EditDraft, type EditProblem } from "./editarModel";
import styles from "./editar.module.css";

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
  const t = useT();
  const [edit, setEdit] = useState<GameEdit | null>(null);
  const [draft, setDraft] = useState<EditDraft | null>(null);
  const [saving, setSaving] = useState(false);
  const [problem, setProblem] = useState<EditProblem | null>(null);
  const pendingPickId = useRef<number | null>(null);
  const formId = useId();

  const trackingOptions = useMemo(
    () =>
      TRACKING_ORDER.map((value) => ({
        value,
        label: t(value === "install_folder" ? "edit_game.track.folder" : `edit_game.track.${value}`),
      })),
    [t],
  );

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

  useEffect(() => {
    pendingPickId.current = null;
  }, [open]);

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
        onError(failure instanceof CallError && failure.error.code === "conflict" ? t("edit_game.not_manual") : t("edit_game.load_error"));
        onClose();
      },
    );
    return () => {
      cancelled = true;
    };
  }, [open, gameId, onClose, onError, t]);

  useEffect(() => {
    return on("system.picked", ({ id, path: pickedPath }) => {
      if (pendingPickId.current === id) {
        pendingPickId.current = null;
        if (pickedPath !== null) {
          setDraft((prev) => (prev ? { ...prev, workingDirectory: pickedPath } : prev));
          setProblem(null);
        }
      }
    });
  }, []);

  const pickFolder = async () => {
    try {
      const res = await call("system.pick", {
        kind: "folder",
        purpose: "edit_game",
      });
      pendingPickId.current = res.id;
    } catch (failure) {
      console.error("system.pick", failure);
    }
  };

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

  const nameError = problem === "name" ? t("edit_game.name_required") : undefined;
  const folderError = problem === "folder" ? t("game_settings.folder_missing", { folder: draft.workingDirectory.trim() }) : undefined;
  const generalError =
    problem === "invalid" ? t("edit_game.invalid") : problem === "manual" ? t("edit_game.not_manual") : problem === "save" ? t("source_ids.save_failed") : undefined;

  return (
    <Dialogo
      open={open}
      onClose={onClose}
      title={t("edit_game.title")}
      closeLabel={t("common.close")}
      size="m"
      actions={
        <>
          <BotaoDesktop variant="outlined" onClick={onClose}>
            {t("common.cancel")}
          </BotaoDesktop>
          <BotaoDesktop type="submit" form={formId} disabled={saving || !isDirty(draft, edit)}>
            {t("common.save")}
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
          label={t("edit_game.name")}
          value={draft.title}
          maxLength={200}
          autoFocus
          error={nameError}
          onChange={(title) => {
            change({ title });
          }}
        />
        <CampoDeTexto
          label={t("edit_game.arguments")}
          value={draft.arguments}
          maxLength={1024}
          help={t("game_settings.none")}
          onChange={(args) => {
            change({ arguments: args });
          }}
        />
        <div className={styles["typingRow"]}>
          <CampoDeTexto
            label={t("game_settings.folder")}
            value={draft.workingDirectory}
            maxLength={1024}
            placeholder={edit.defaultFolder ?? ""}
            help={t("game_settings.folder_hint")}
            error={folderError}
            onChange={(workingDirectory) => {
              change({ workingDirectory });
            }}
          />
          <BotaoDesktop
            type="button"
            variant="outlined"
            onClick={() => {
              void pickFolder();
            }}
          >
            {t("edit_game.choose")}
          </BotaoDesktop>
        </div>
        <GrupoDeConfiguracao>
          <LinhaDeConfiguracao
            title={t("edit_game.tracking")}
            control={
              <CampoDeSelecao
                label={t("edit_game.tracking")}
                options={trackingOptions}
                value={draft.tracking}
                onChange={(tracking) => {
                  change({ tracking });
                }}
              />
            }
          />
        </GrupoDeConfiguracao>
        {generalError && (
          <p className={styles["info"]} role="alert">
            {generalError}
          </p>
        )}
      </form>
    </Dialogo>
  );
}
