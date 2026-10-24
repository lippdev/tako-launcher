import { useState } from "react";
import { call } from "../bridge";
import type { Collection } from "../bridge/generated/Collection";
import type { Tag } from "../bridge/generated/Tag";
import { Icon } from "../components/Icon/Icon";
import { MenuDeContexto, type MenuEntry } from "../components/MenuDeContexto/MenuDeContexto";
import { PainelLateral } from "../components/PainelLateral/PainelLateral";
import { STATUS_IDS } from "./uiState";
import { libraryStrings as s } from "./strings";
import styles from "./LibraryControls.module.css";

export function LibraryBulkActions({
  selection,
  allFavorite,
  collections,
  tags,
  clear,
  notify,
}: {
  selection: ReadonlySet<string>;
  allFavorite: boolean;
  collections: Collection[];
  tags: Tag[];
  clear: () => void;
  notify: (error: boolean) => void;
}) {
  const [busy, setBusy] = useState(false);
  const [menu, setMenu] = useState<{
    anchor: HTMLElement;
    kind: "status" | "collection";
  } | null>(null);
  const [tagsOpen, setTagsOpen] = useState(false);
  const [tag, setTag] = useState("");
  const gameIds = [...selection];
  const run = (action: () => Promise<unknown>) => {
    if (busy) return;
    setBusy(true);
    void action()
      .then(() => {
        notify(false);
      })
      .catch((error: unknown) => {
        console.error("biblioteca: ação", error);
        notify(true);
      })
      .finally(() => {
        setBusy(false);
      });
  };
  const entries: MenuEntry[] =
    menu?.kind === "status"
      ? [
          {
            id: "none",
            label: s.noStatus,
            onSelect: () => {
              run(() =>
                call("game.status.set", {
                  gameIds,
                  status: null,
                }),
              );
            },
          },
          ...STATUS_IDS.map((status) => ({
            id: status,
            label: s.statuses[status],
            onSelect: () => {
              run(() => call("game.status.set", { gameIds, status }));
            },
          })),
        ]
      : collections
          .filter((c) => !c.criteria)
          .flatMap((c): MenuEntry[] => [
            { type: "header", label: c.name },
            {
              id: `add-${c.id}`,
              label: s.addCollection,
              onSelect: () => {
                run(() =>
                  call("collections.add_games", {
                    collectionId: c.id,
                    gameIds,
                  }),
                );
              },
            },
            {
              id: `remove-${c.id}`,
              label: s.removeCollection,
              onSelect: () => {
                run(() =>
                  call("collections.remove_games", {
                    collectionId: c.id,
                    gameIds,
                  }),
                );
              },
            },
          ]);
  return (
    <>
      <div className={styles["actions"]} role="toolbar" aria-label={s.selectionActions} aria-busy={busy}>
        <button
          disabled={busy || !collections.some((c) => !c.criteria)}
          title={s.collection}
          aria-label={s.collection}
          aria-haspopup="menu"
          onClick={(e) => {
            setMenu({
              anchor: e.currentTarget,
              kind: "collection",
            });
          }}
        >
          <Icon name="folder" />
        </button>
        <button
          disabled={busy}
          title={s.tags}
          aria-label={s.tags}
          aria-haspopup="dialog"
          onClick={() => {
            setTagsOpen(true);
          }}
        >
          <Icon name="list" />
        </button>
        <button
          disabled={busy}
          title={s.status}
          aria-label={s.status}
          aria-haspopup="menu"
          onClick={(e) => {
            setMenu({ anchor: e.currentTarget, kind: "status" });
          }}
        >
          <Icon name="check" />
        </button>
        <button
          disabled={busy}
          title={allFavorite ? s.unfavorite : s.favorite}
          aria-label={allFavorite ? s.unfavorite : s.favorite}
          onClick={() => {
            run(() =>
              call("game.favorite.set", {
                gameIds,
                favorite: !allFavorite,
              }),
            );
          }}
        >
          <Icon name="star" filled={allFavorite} />
        </button>
        <button title={s.close} aria-label={s.close} onClick={clear}>
          <Icon name="x" />
        </button>
      </div>
      <MenuDeContexto
        open={menu !== null}
        onClose={() => {
          setMenu(null);
        }}
        anchor={menu?.anchor ?? null}
        entries={entries}
        title={menu?.kind === "status" ? s.status : s.collection}
      />
      <PainelLateral
        open={tagsOpen}
        onClose={() => {
          setTagsOpen(false);
        }}
        title={s.tags}
        closeLabel={s.close}
      >
        <div className={styles["fields"]}>
          <input
            className={styles["tagInput"]}
            aria-label={s.tags}
            placeholder={s.tagPlaceholder}
            list="library-tag-suggestions"
            value={tag}
            onChange={(e) => {
              setTag(e.target.value);
            }}
          />
          <datalist id="library-tag-suggestions">
            {tags.map((t) => (
              <option key={t.name} value={t.name} />
            ))}
          </datalist>
          <div className={styles["footer"]}>
            <button
              disabled={busy || !tag.trim()}
              onClick={() => {
                run(() =>
                  call("game.tags.set", {
                    gameIds,
                    tags: [tag.trim()],
                    on: false,
                  }),
                );
              }}
            >
              {s.removeTags}
            </button>
            <button
              disabled={busy || !tag.trim()}
              onClick={() => {
                run(() =>
                  call("game.tags.set", {
                    gameIds,
                    tags: [tag.trim()],
                    on: true,
                  }),
                );
              }}
            >
              {s.addTags}
            </button>
          </div>
        </div>
      </PainelLateral>
    </>
  );
}
