import { BotaoDesktop } from "../components/BotaoDesktop/BotaoDesktop";
import { Chave } from "../components/Chave/Chave";
import { CampoDeSelecao } from "../components/CampoDeSelecao/CampoDeSelecao";
import { PainelLateral } from "../components/PainelLateral/PainelLateral";
import type { Collection } from "../bridge/generated/Collection";
import type { Tag } from "../bridge/generated/Tag";
import { useT } from "../i18n";
import { defaultLibraryCriteria, SORT_IDS, STATUS_IDS, STORE_IDS, type LibraryCriteria } from "./uiState";
import styles from "./LibraryControls.module.css";

export function filterCount(c: LibraryCriteria) {
  return (
    c.stores.length +
    c.statuses.length +
    c.tags.length +
    Number(c.noStatus) +
    Number(c.installed !== null) +
    Number(c.collection !== null)
  );
}

export function collectionOptions(collections: Collection[], t: (key: string, vars?: Record<string, string | number>) => string) {
  return [
    { value: "", label: t("library.all_collections") },
    ...collections.map((c) => ({
      value: c.id,
      label: c.criteria ? c.name : t("library.collection_label", { name: c.name, count: c.gameIds.length }),
    })),
  ];
}

export function LibraryFilters({
  open,
  onClose,
  criteria: c,
  update,
  collections,
  tags,
  canSaveCollection,
  onSaveCollection,
  onManageCollections,
}: {
  open: boolean;
  onClose: () => void;
  criteria: LibraryCriteria;
  update: (patch: Partial<LibraryCriteria>) => void;
  collections: Collection[];
  tags: Tag[];
  /** Há busca, filtro ou ordenação a guardar como coleção automática. */
  canSaveCollection: boolean;
  onSaveCollection: () => void;
  onManageCollections: () => void;
}) {
  const t = useT();
  const count = filterCount(c);
  return (
    <PainelLateral
      open={open}
      onClose={onClose}
      title={t("library.filters")}
      closeLabel={t("common.close")}
      count={String(count)}
      footer={
        <div className={styles["footer"]}>
          {(count > 0 || c.sort !== defaultLibraryCriteria.sort || c.reversed) && (
            <button
              onClick={() => {
                update({
                  ...defaultLibraryCriteria,
                  filter: c.filter,
                  search: c.search,
                });
              }}
            >
              {t("common.clear")}
            </button>
          )}
          <button onClick={onClose}>{t("common.apply")}</button>
        </div>
      }
    >
      <div className={styles["fields"]}>
        <div className={styles["field"]}>
          <span>{t("filters.sort_by")}</span>
          <CampoDeSelecao
            label={t("filters.sort_by")}
            value={c.sort}
            options={SORT_IDS.map((value) => ({
              value,
              label: t(`library.sort.${value}`),
            }))}
            onChange={(sort) => {
              update({ sort });
            }}
          />
        </div>
        <div className={styles["field"]}>
          <span id="library-reversed">{t("library.reversed")}</span>
          <Chave
            checked={c.reversed}
            aria-labelledby="library-reversed"
            onChange={(reversed) => {
              update({ reversed });
            }}
          />
        </div>
        <div className={styles["field"]}>
          <span>{t("desktop.group.collection")}</span>
          <CampoDeSelecao
            label={t("desktop.group.collection")}
            value={c.collection ?? ""}
            options={collectionOptions(collections, t)}
            onChange={(collection) => {
              update({ collection: collection || null });
            }}
          />
        </div>
        <div className={styles["field"]}>
          <span>{t("desktop.group.install")}</span>
          <CampoDeSelecao
            label={t("desktop.group.install")}
            value={c.installed === null ? "all" : c.installed ? "yes" : "no"}
            options={[
              { value: "all", label: t("library.install.any") },
              { value: "yes", label: t("library.install.installed") },
              { value: "no", label: t("library.install.not_installed") },
            ]}
            onChange={(v) => {
              update({
                installed: v === "all" ? null : v === "yes",
              });
            }}
          />
        </div>
        <div className={styles["field"]}>
          <span>{t("desktop.group.store")}</span>
          <CampoDeSelecao
            label={t("desktop.group.store")}
            value={c.stores[0] ?? ""}
            options={[
              { value: "", label: t("library.install.any") },
              ...STORE_IDS.map((value) => ({ value, label: t(`sources.store.${value}`) })),
            ]}
            onChange={(store) => {
              update({ stores: store ? [store] : [] });
            }}
          />
        </div>
        <div className={styles["field"]}>
          <span>{t("desktop.group.status")}</span>
          <CampoDeSelecao
            label={t("desktop.group.status")}
            value={c.noStatus ? "none" : (c.statuses[0] ?? "")}
            options={[
              { value: "", label: t("library.install.any") },
              { value: "none", label: t("library.no_status") },
              ...STATUS_IDS.map((value) => ({ value, label: t(`library.status.${value}`) })),
            ]}
            onChange={(status) => {
              update({
                noStatus: status === "none",
                statuses: status && status !== "none" ? [status] : [],
              });
            }}
          />
        </div>
        {tags.length > 0 && (
          <div className={styles["field"]}>
            <span>{t("desktop.group.tag")}</span>
            <CampoDeSelecao
              label={t("desktop.group.tag")}
              value={c.tags[0] ?? ""}
              options={[
                { value: "", label: t("library.install.any") },
                ...tags.map((tag) => ({ value: tag.name, label: `${tag.name} · ${String(tag.count)}` })),
              ]}
              onChange={(tag) => {
                update({ tags: tag ? [tag] : [] });
              }}
            />
          </div>
        )}
        <div className={styles["field"]}>
          <span>{t("collections.title")}</span>
          <div className={styles["collectionActions"]}>
            {canSaveCollection && (
              <BotaoDesktop variant="outlined" title={t("smart.save_desc")} onClick={onSaveCollection}>
                {t("smart.save")}
              </BotaoDesktop>
            )}
            <BotaoDesktop variant="outlined" onClick={onManageCollections}>
              {t("desktop.collections_manage")}
            </BotaoDesktop>
          </div>
        </div>
      </div>
    </PainelLateral>
  );
}
