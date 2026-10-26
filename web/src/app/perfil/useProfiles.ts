import { useCallback, useEffect, useRef, useState } from "react";
import { CallError, call } from "../../bridge";
import type { ErrorCode } from "../../bridge/generated/ErrorCode";
import type { ProfilesList } from "../../bridge/generated/ProfilesList";

/** O que uma mudança nos perfis deu: a lista nova ou o código do erro (`null` = falha que não veio do núcleo). */
export type ProfilesResult = { ok: true; list: ProfilesList } | { ok: false; code: ErrorCode | null };

export interface Profiles {
  /** A lista do núcleo; `null` até a primeira leitura (ou se ela falhou). */
  list: ProfilesList | null;
  /** Uma mudança está a caminho: os botões esperam. */
  busy: boolean;
  create: (name: string) => Promise<ProfilesResult>;
  rename: (profileId: string, name: string) => Promise<ProfilesResult>;
  remove: (profileId: string) => Promise<ProfilesResult>;
  /** Grava o perfil da próxima abertura (os dados em uso só trocam ao reabrir o Tako). */
  switchTo: (profileId: string) => Promise<ProfilesResult>;
  setAskOnStart: (on: boolean) => Promise<ProfilesResult>;
  reload: () => void;
}

/** A lista de perfis locais (UX-004) e as mudanças nela, pela ponte. Cada mudança devolve a lista já atualizada. */
export function useProfiles(active = true): Profiles {
  const [list, setList] = useState<ProfilesList | null>(null);
  const [busy, setBusy] = useState(false);
  const alive = useRef(true);

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  const reload = useCallback(() => {
    call("profiles.list").then(
      (next) => {
        if (alive.current) setList(next);
      },
      (error: unknown) => {
        console.error("profiles.list", error);
      },
    );
  }, []);

  useEffect(() => {
    if (active) reload();
  }, [active, reload]);

  const run = useCallback(async (change: () => Promise<ProfilesList>): Promise<ProfilesResult> => {
    setBusy(true);
    try {
      const next = await change();
      if (alive.current) setList(next);
      return { ok: true, list: next };
    } catch (error) {
      if (!(error instanceof CallError)) console.error("profiles", error);
      return { ok: false, code: error instanceof CallError ? error.error.code : null };
    } finally {
      if (alive.current) setBusy(false);
    }
  }, []);

  return {
    list,
    busy,
    reload,
    create: useCallback((name) => run(() => call("profiles.create", { name })), [run]),
    rename: useCallback((profileId, name) => run(() => call("profiles.rename", { profileId, name })), [run]),
    remove: useCallback((profileId) => run(() => call("profiles.delete", { profileId })), [run]),
    switchTo: useCallback((profileId) => run(() => call("profiles.switch", { profileId })), [run]),
    setAskOnStart: useCallback((on) => run(() => call("profiles.ask.set", { on })), [run]),
  };
}
