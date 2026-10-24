import { Chave } from "../components/Chave/Chave";
import { CampoDeSelecao } from "../components/CampoDeSelecao/CampoDeSelecao";
import { PainelLateral } from "../components/PainelLateral/PainelLateral";
import type { Collection } from "../bridge/generated/Collection";
import type { Tag } from "../bridge/generated/Tag";
import { defaultLibraryCriteria, SORT_IDS, STATUS_IDS, STORE_IDS, type LibraryCriteria } from "./uiState";
import { libraryStrings as s } from "./strings";
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
export function collectionOptions(collections: Collection[]) {
  return [
    { value: "", label: s.allCollections },
    ...collections.map((c) => ({
      value: c.id,
      label: s.collectionLabel(c.name, c.criteria ? null : c.gameIds.length),
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
}: {
  open: boolean;
  onClose: () => void;
  criteria: LibraryCriteria;
  update: (patch: Partial<LibraryCriteria>) => void;
  collections: Collection[];
  tags: Tag[];
}) {
  const count = filterCount(c);
  return (
    <PainelLateral
      open={open}
      onClose={onClose}
      title={s.filters}
      closeLabel={s.close}
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
              {s.clear}
            </button>
          )}
          <button onClick={onClose}>{s.apply}</button>
        </div>
      }
    >
      <div className={styles["fields"]}>
        <div className={styles["field"]}>
          <span>{s.sort}</span>
          <CampoDeSelecao
            label={s.sort}
            value={c.sort}
            options={SORT_IDS.map((value) => ({
              value,
              label: s.sorts[value],
            }))}
            onChange={(sort) => {
              update({ sort });
            }}
          />
        </div>
        <div className={styles["field"]}>
          <span id="library-reversed">{s.reversed}</span>
          <Chave
            checked={c.reversed}
            aria-labelledby="library-reversed"
            onChange={(reversed) => {
              update({ reversed });
            }}
          />
        </div>
        <div className={styles["field"]}>
          <span>{s.collection}</span>
          <CampoDeSelecao
            label={s.collection}
            value={c.collection ?? ""}
            options={collectionOptions(collections)}
            onChange={(collection) => {
              update({ collection: collection || null });
            }}
          />
        </div>
        <div className={styles["field"]}>
          <span>{s.installed}</span>
          <CampoDeSelecao
            label={s.installed}
            value={c.installed === null ? "all" : c.installed ? "yes" : "no"}
            options={[
              { value: "all", label: s.any },
              { value: "yes", label: s.installedOnly },
              { value: "no", label: s.notInstalled },
            ]}
            onChange={(v) => {
              update({
                installed: v === "all" ? null : v === "yes",
              });
            }}
          />
        </div>
        <div className={styles["field"]}>
          <span>{s.store}</span>
          <CampoDeSelecao
            label={s.store}
            value={c.stores[0] ?? ""}
            options={[
              { value: "", label: s.any },
              ...STORE_IDS.map((value) => ({ value, label: s.stores[value] })),
            ]}
            onChange={(store) => {
              update({ stores: store ? [store] : [] });
            }}
          />
        </div>
        <div className={styles["field"]}>
          <span>{s.status}</span>
          <CampoDeSelecao
            label={s.status}
            value={c.noStatus ? "none" : (c.statuses[0] ?? "")}
            options={[
              { value: "", label: s.any },
              { value: "none", label: s.noStatus },
              ...STATUS_IDS.map((value) => ({ value, label: s.statuses[value] })),
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
            <span>{s.tags}</span>
            <CampoDeSelecao
              label={s.tags}
              value={c.tags[0] ?? ""}
              options={[
                { value: "", label: s.any },
                ...tags.map((tag) => ({ value: tag.name, label: `${tag.name} · ${String(tag.count)}` })),
              ]}
              onChange={(tag) => {
                update({ tags: tag ? [tag] : [] });
              }}
            />
          </div>
        )}
      </div>
    </PainelLateral>
  );
}
