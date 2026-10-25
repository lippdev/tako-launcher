import { useCallback, useEffect, useMemo, useState } from "react";
import { call, on } from "../../bridge";
import {
  EditorDeEtiquetas,
  type EditorDeEtiquetasLabels,
  type SugestaoDeEtiqueta,
} from "../../components/EditorDeEtiquetas/EditorDeEtiquetas";
import { useT } from "../../i18n";

/** Todas as etiquetas da biblioteca (`tags.list`), lidas de novo a cada `library.changed`. */
function useTagList(): SugestaoDeEtiqueta[] {
  const [tags, setTags] = useState<SugestaoDeEtiqueta[]>([]);
  const read = useCallback(() => {
    call("tags.list").then(
      (list) => {
        setTags(list.map((tag) => ({ nome: tag.name, jogos: tag.count })));
      },
      (error: unknown) => {
        // Sem a lista o campo ainda cria e aplica; só não sugere as que já existem.
        console.error("tags.list", error);
      },
    );
  }, []);
  useEffect(() => {
    read();
    return on("library.changed", read);
  }, [read]);
  return tags;
}

interface EditorDasEtiquetasProps {
  gameId: string;
  /** As etiquetas do jogo. */
  tags: readonly string[];
  /** A etiqueta entrou ou saiu: a página lê os detalhes de novo. */
  onChanged: () => void;
  onError: (text: string) => void;
}

/** O editor de etiquetas da página do jogo, ligado ao núcleo: aplica e tira com `game.tags.set`. */
export function EditorDasEtiquetas({ gameId, tags, onChanged, onError }: EditorDasEtiquetasProps) {
  const sugestoes = useTagList();
  const t = useT();

  const labels = useMemo<EditorDeEtiquetasLabels>(
    () => ({
      list: t("tags.list"),
      add: t("tags.placeholder"),
      field: t("tags.field"),
      title: t("desktop.info.tags"),
      remove: (name: string) => t("tags.remove", { name }),
      create: (name: string) => t("tags.create", { name }),
      games: (n: number) => t("common.games", { n }),
    }),
    [t],
  );

  const change = (name: string, on: boolean) => {
    call("game.tags.set", { gameIds: [gameId], tags: [name], on }).then(onChanged, (error: unknown) => {
      console.error("game.tags.set", error);
      onError(t("tags.error"));
    });
  };
  return (
    <EditorDeEtiquetas
      tags={tags}
      suggestions={sugestoes}
      labels={labels}
      size="sm"
      align="end"
      onAdd={(name) => {
        change(name, true);
      }}
      onRemove={(name) => {
        change(name, false);
      }}
    />
  );
}
