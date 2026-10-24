import { useState } from "react";
import { call } from "../../bridge";
import { PerfilColecoes, type CollectionItem } from "../../components/PerfilColecoes/PerfilColecoes";
import { useT } from "../../i18n";
import { profileStrings as s } from "../strings";
import { collectionDetail, collectionsSummary } from "./perfilModel";
import { NovaColecaoDialogo } from "./NovaColecaoDialogo";
import { useBridgeData } from "./useBridgeData";
import { useCapas } from "./useCapas";
import styles from "./TelaPerfil.module.css";

interface ColecoesDoPerfilProps {
  /** Abrir a biblioteca já filtrada na coleção. */
  onOpenCollection: (id: string) => void;
}

const loadCollections = () => call("profile.collections");

/** Perfil › Coleções (README do PerfilColecoes): as do usuário e as automáticas, com o mosaico das capas. */
export function ColecoesDoPerfil({ onOpenCollection }: ColecoesDoPerfilProps) {
  const t = useT();
  const cover = useCapas(120, 160);
  const { data, status, reload } = useBridgeData(loadCollections);
  const [creating, setCreating] = useState(false);

  if (status === "error") {
    return (
      <div className={styles["aviso"]} role="alert">
        {s.loadError}
        <button type="button" className={styles["retry"]} onClick={reload}>
          {s.retry}
        </button>
      </div>
    );
  }

  const items: CollectionItem[] = (data?.collections ?? []).map((collection) => ({
    id: collection.id,
    name: collection.name,
    detail: collectionDetail(collection.gameCount, collection.installedCount, t),
    automatic: collection.automatic,
    covers: collection.covers.flatMap((game) => cover(game) ?? []),
    onClick: () => {
      onOpenCollection(collection.id);
    },
  }));

  return (
    <>
      {data && (
        <PerfilColecoes
          summary={collectionsSummary(data.collections.length, data.gameCount, t)}
          toolbarLabel={s.collectionsToolbar}
          gridLabel={s.collectionsGrid}
          actionsLabel={s.collectionActions}
          autoBadgeLabel={t("profile.auto")}
          collections={items}
          newCollection={{
            label: t("collections.new"),
            onClick: () => {
              setCreating(true);
            },
          }}
        />
      )}
      <NovaColecaoDialogo
        open={creating}
        onClose={() => {
          setCreating(false);
        }}
      />
    </>
  );
}
