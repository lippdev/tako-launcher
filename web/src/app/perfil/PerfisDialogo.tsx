import { useState, type FormEvent, type ReactNode } from "react";
import type { ProfileEntry } from "../../bridge/generated/ProfileEntry";
import type { ProfilesList } from "../../bridge/generated/ProfilesList";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT, type TFunction } from "../../i18n";
import { profileStrings as s } from "../strings";
import { LinhaChave } from "../settings/Linhas";
import { profileName } from "./perfilModel";
import type { Profiles, ProfilesResult } from "./useProfiles";
import styles from "./PerfisDialogo.module.css";

/** Em que ponto da gestão o diálogo está; Esc e ✕ voltam um passo antes de fechar. */
export type PerfisView =
  | { kind: "list" }
  | { kind: "manage"; id: string }
  | { kind: "name"; id: string | null }
  | { kind: "remove"; id: string };

interface PerfisDialogoProps {
  open: boolean;
  onClose: () => void;
  /** Por onde começa: a lista (`null`, Trocar perfil) ou a gestão de um perfil (Editar perfil: o perfil em uso). */
  manageId: string | null;
  profiles: Profiles;
  /** O perfil da próxima abertura mudou: a tela avisa que vale ao reabrir o Tako. */
  onSwitched: (name: string) => void;
  /** A lista mudou (nome, perfil novo ou apagado): a tela que mostra o perfil lê de novo. */
  onChanged: () => void;
}

/** O texto do erro de uma mudança nos perfis, pelos códigos do núcleo (`profiles.*`). */
function errorText(result: Extract<ProfilesResult, { ok: false }>, list: ProfilesList | null, t: TFunction): string {
  switch (result.code) {
    case "bad_request":
      return t("profiles.invalid_name");
    case "conflict":
      // Nome repetido e perfis demais voltam como `conflict`; a lista diz qual dos dois foi.
      return list && list.profiles.length >= list.maxProfiles ? t("profiles.too_many") : t("profiles.duplicate_name");
    default:
      return t("profiles.save_failed");
  }
}

/**
 * Trocar perfil e Editar perfil (UX-004): a lista de perfis com "Novo perfil" e "Perguntar ao abrir", e, por perfil,
 * Usar, Renomear e Apagar, como no app Slint (`tako-ui/src/shell/profiles.rs`). Usar grava o perfil da próxima
 * abertura: o núcleo abre os dados de um perfil só por vez.
 */
export function PerfisDialogo({ open, onClose, manageId, profiles, onSwitched, onChanged }: PerfisDialogoProps) {
  const t = useT();
  const [view, setView] = useState<PerfisView>({ kind: "list" });
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const { list, busy } = profiles;

  // Cada abertura recomeça do ponto pedido, sem sobra da vez anterior (e sem trocar o conteúdo durante a saída).
  const [wasOpen, setWasOpen] = useState(false);
  if (open !== wasOpen) {
    setWasOpen(open);
    if (open) {
      setView(manageId === null ? { kind: "list" } : { kind: "manage", id: manageId });
      setError(null);
    }
  }

  const find = (id: string): ProfileEntry | undefined => list?.profiles.find((p) => p.id === id);

  function go(next: PerfisView) {
    setError(null);
    if (next.kind === "name") {
      setName(next.id === null ? "" : (find(next.id)?.name ?? ""));
    }
    setView(next);
  }

  function back() {
    setError(null);
    if (view.kind === "list" || (view.kind === "manage" && manageId !== null)) onClose();
    else if (view.kind === "name" && view.id !== null) setView({ kind: "manage", id: view.id });
    else if (view.kind === "remove") setView({ kind: "manage", id: view.id });
    else setView({ kind: "list" });
  }

  async function submitName(event: FormEvent) {
    event.preventDefault();
    if (view.kind !== "name" || busy) return;
    const result = view.id === null ? await profiles.create(name) : await profiles.rename(view.id, name);
    if (!result.ok) {
      setError(errorText(result, list, t));
      return;
    }
    onChanged();
    setView(view.id === null ? { kind: "list" } : { kind: "manage", id: view.id });
  }

  async function use(id: string) {
    const result = await profiles.switchTo(id);
    if (!result.ok) {
      setError(errorText(result, list, t));
      return;
    }
    const entry = result.list.profiles.find((p) => p.id === id);
    onClose();
    // Escolher o perfil que já está em uso só fecha; outro vale na próxima abertura.
    if (result.list.restartRequired && entry) onSwitched(profileName(entry, t));
  }

  async function remove(id: string) {
    const result = await profiles.remove(id);
    if (!result.ok) {
      setError(errorText(result, list, t));
      return;
    }
    onChanged();
    setView({ kind: "list" });
  }

  const cancel = (
    <BotaoDesktop variant="secondary" size="sm" onClick={back}>
      {t("common.cancel")}
    </BotaoDesktop>
  );
  const alert = error && (
    <p className={styles["error"]} role="alert">
      {error}
    </p>
  );

  let title = t("profiles.title");
  let actions: ReactNode = null;
  let body: ReactNode;

  if (view.kind === "list") {
    const full = list !== null && list.profiles.length >= list.maxProfiles;
    body = (
      <>
        <p className={styles["text"]}>{t("profiles.text")}</p>
        <GrupoDeConfiguracao>
          {list?.profiles.map((profile) => (
            <LinhaDeConfiguracao
              key={profile.id}
              title={profileName(profile, t)}
              {...(profile.inUse
                ? { description: t("profiles.in_use") }
                : profile.next
                  ? { description: s.nextOpen }
                  : {})}
              onClick={() => {
                go({ kind: "manage", id: profile.id });
              }}
            />
          ))}
          <LinhaDeConfiguracao
            title={t("profiles.new")}
            description={t("profiles.new_desc")}
            onClick={() => {
              if (!full) go({ kind: "name", id: null });
            }}
          />
          <LinhaChave
            title={t("profiles.ask")}
            description={t("profiles.ask_desc")}
            checked={list?.askOnStart ?? false}
            disabled={busy || list === null}
            onChange={(on) => {
              void profiles.setAskOnStart(on).then((result) => {
                if (!result.ok) setError(errorText(result, list, t));
              });
            }}
          />
        </GrupoDeConfiguracao>
        {full && <p className={styles["note"]}>{t("profiles.too_many")}</p>}
        {alert}
      </>
    );
  } else if (view.kind === "manage") {
    const profile = find(view.id);
    title = profile ? profileName(profile, t) : t("profiles.title");
    const canRemove = profile !== undefined && !profile.main && !profile.inUse && !profile.next;
    body = (
      <>
        {profile?.main && <p className={styles["text"]}>{t("profiles.main_desc")}</p>}
        <GrupoDeConfiguracao>
          {profile && !profile.next && (
            <LinhaDeConfiguracao
              title={t("profiles.use")}
              onClick={() => {
                void use(profile.id);
              }}
            />
          )}
          <LinhaDeConfiguracao
            title={t("profiles.rename")}
            onClick={() => {
              go({ kind: "name", id: view.id });
            }}
          />
          {canRemove && (
            <LinhaDeConfiguracao
              danger
              title={t("profiles.remove")}
              onClick={() => {
                go({ kind: "remove", id: view.id });
              }}
            />
          )}
        </GrupoDeConfiguracao>
        {alert}
      </>
    );
    actions = cancel;
  } else if (view.kind === "name") {
    title = view.id === null ? t("profiles.new") : t("profiles.rename");
    body = (
      <form id="perfil-nome" onSubmit={(event) => void submitName(event)}>
        <CampoDeTexto
          label={t("profiles.name_hint", { max: list?.maxNameChars ?? 32 })}
          value={name}
          onChange={(value) => {
            setName(value);
            setError(null);
          }}
          maxLength={list?.maxNameChars ?? 32}
          autoFocus
          autoComplete="off"
          {...(error ? { error } : {})}
        />
      </form>
    );
    actions = (
      <>
        {cancel}
        <BotaoDesktop variant="primary" size="sm" type="submit" form="perfil-nome" disabled={busy || name.trim() === ""}>
          {t("common.save")}
        </BotaoDesktop>
      </>
    );
  } else {
    const profile = find(view.id);
    title = t("profiles.remove_title", { name: profile ? profileName(profile, t) : "" });
    body = (
      <>
        <p className={styles["text"]}>{t("profiles.remove_text")}</p>
        {alert}
      </>
    );
    actions = (
      <>
        {cancel}
        <BotaoDesktop
          variant="primary"
          size="sm"
          disabled={busy}
          onClick={() => {
            void remove(view.id);
          }}
        >
          {t("profiles.remove")}
        </BotaoDesktop>
      </>
    );
  }

  return (
    <Dialogo open={open} onClose={back} size="m" closeLabel={t("common.close")} title={title} actions={actions}>
      {body}
    </Dialogo>
  );
}
