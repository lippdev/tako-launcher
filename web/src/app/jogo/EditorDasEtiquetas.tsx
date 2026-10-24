import { useCallback, useEffect, useState } from "react";
import { call, on } from "../../bridge";
import { EditorDeEtiquetas, type EditorDeEtiquetasRotulos, type SugestaoDeEtiqueta } from "../../components/EditorDeEtiquetas/EditorDeEtiquetas";
import { tagEditorStrings as s } from "../strings";

const ROTULOS: EditorDeEtiquetasRotulos = {
  lista: s.list,
  adicionar: s.add,
  campo: s.field,
  titulo: s.menuTitle,
  tirar: s.remove,
  criar: s.create,
  jogos: s.games,
};

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
  const change = (name: string, on: boolean) => {
    call("game.tags.set", { gameIds: [gameId], tags: [name], on }).then(onChanged, (error: unknown) => {
      console.error("game.tags.set", error);
      onError(s.error);
    });
  };
  return (
    <EditorDeEtiquetas
      etiquetas={tags}
      sugestoes={sugestoes}
      rotulos={ROTULOS}
      tamanho="sm"
      alinhamento="fim"
      onAdicionar={(name) => {
        change(name, true);
      }}
      onRemover={(name) => {
        change(name, false);
      }}
    />
  );
}
