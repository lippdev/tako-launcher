import { useEffect, useState } from "react";
import { call, on } from "../../bridge";
import type { ArtCandidateDto } from "../../bridge/generated/ArtCandidateDto";
import type { ArtKindDto } from "../../bridge/generated/ArtKindDto";
import {
  SeletorDeImagens,
  type OpcaoDeImagem,
  type TipoDeImagem,
} from "../../components/SeletorDeImagens/SeletorDeImagens";

export interface DialogoSeletorDeImagensProps {
  gameId: string;
  gameTitle: string;
  open: boolean;
  tipoInicial?: TipoDeImagem;
  onClose: () => void;
  onImageChanged?: () => void;
}

const TIPO_TO_KIND: Record<TipoDeImagem, ArtKindDto> = {
  capa: "cover",
  fundo: "hero",
  logo: "logo",
  icone: "icon",
};

const KIND_TO_TIPO: Record<ArtKindDto, TipoDeImagem> = {
  cover: "capa",
  hero: "fundo",
  logo: "logo",
  icon: "icone",
};

function candidateToOption(c: ArtCandidateDto): OpcaoDeImagem {
  const item: OpcaoDeImagem = {
    id: c.id,
    url: c.url,
    source: c.source,
    current: c.current,
  };
  if (c.dimensions) item.dimensions = c.dimensions;
  if (c.style) item.style = c.style;
  return item;
}

export function DialogoSeletorDeImagens({
  gameId,
  gameTitle,
  open,
  tipoInicial = "capa",
  onClose,
  onImageChanged,
}: DialogoSeletorDeImagensProps) {
  const [activeType, setActiveType] = useState<TipoDeImagem>(tipoInicial);
  const [optionsByType, setOptionsByType] = useState<Partial<Record<TipoDeImagem, readonly OpcaoDeImagem[]>>>({});
  const [loadingByType, setLoadingByType] = useState<Partial<Record<TipoDeImagem, boolean>>>({});
  const [emptyByType, setEmptyByType] = useState<Partial<Record<TipoDeImagem, boolean>>>({});

  useEffect(() => {
    if (!open) return;

    const stop = on("art.candidates.ready", (payload) => {
      if (payload.gameId !== gameId) return;
      const tipo = KIND_TO_TIPO[payload.kind];
      if (!tipo) return;
      const options = payload.candidates.map(candidateToOption);
      setOptionsByType((prev) => ({ ...prev, [tipo]: options }));
      setEmptyByType((prev) => ({ ...prev, [tipo]: options.length === 0 }));
      setLoadingByType((prev) => ({ ...prev, [tipo]: false }));
    });

    return () => {
      stop();
    };
  }, [open, gameId]);

  useEffect(() => {
    if (!open) return;
    if (optionsByType[activeType] !== undefined) return;

    let cancelled = false;

    call("art.candidates", {
      gameId,
      kind: TIPO_TO_KIND[activeType],
    })
      .then((view) => {
        if (cancelled) return;
        if (!view.loading) {
          const options = view.candidates.map(candidateToOption);
          setOptionsByType((prev) => ({ ...prev, [activeType]: options }));
          setEmptyByType((prev) => ({ ...prev, [activeType]: options.length === 0 }));
          setLoadingByType((prev) => ({ ...prev, [activeType]: false }));
        } else if (view.candidates.length > 0) {
          const options = view.candidates.map(candidateToOption);
          setOptionsByType((prev) => ({ ...prev, [activeType]: options }));
          setEmptyByType((prev) => ({ ...prev, [activeType]: false }));
        }
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        console.error("erro ao carregar candidatos de arte", err);
        setOptionsByType((prev) => ({ ...prev, [activeType]: [] }));
        setEmptyByType((prev) => ({ ...prev, [activeType]: true }));
        setLoadingByType((prev) => ({ ...prev, [activeType]: false }));
      });

    return () => {
      cancelled = true;
    };
  }, [open, gameId, activeType, optionsByType]);

  const handleChangeType = (type: TipoDeImagem) => {
    setActiveType(type);
    if (optionsByType[type] === undefined) {
      setLoadingByType((prev) => ({ ...prev, [type]: true }));
    }
  };

  const handleChoose = async (option: OpcaoDeImagem, type: TipoDeImagem) => {
    try {
      await call("art.choose", {
        gameId,
        kind: TIPO_TO_KIND[type],
        candidateId: option.id,
      });
      onImageChanged?.();
      onClose();
    } catch (err) {
      console.error("erro ao escolher arte", err);
    }
  };

  const handleResetToOriginal = async () => {
    try {
      await call("art.reset", {
        gameId,
        kind: TIPO_TO_KIND[activeType],
      });
      onImageChanged?.();
      onClose();
    } catch (err) {
      console.error("erro ao restaurar arte original", err);
    }
  };

  const effectiveLoadingByType: Partial<Record<TipoDeImagem, boolean>> = {
    ...loadingByType,
    ...(open && optionsByType[activeType] === undefined ? { [activeType]: true } : {}),
  };

  return (
    <SeletorDeImagens
      open={open}
      onClose={onClose}
      onChoose={(option, type) => {
        void handleChoose(option, type);
      }}
      gameTitle={gameTitle}
      activeType={activeType}
      onChangeType={handleChangeType}
      optionsByType={optionsByType}
      loadingByType={effectiveLoadingByType}
      emptyByType={emptyByType}
      onResetToOriginal={() => {
        void handleResetToOriginal();
      }}
    />
  );
}
