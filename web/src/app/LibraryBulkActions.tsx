import { useState } from "react";
import { call } from "../bridge";
import type { Collection } from "../bridge/generated/Collection";
import type { Tag } from "../bridge/generated/Tag";
import { Icon } from "../components/Icon/Icon";
import { MenuDeContexto, type MenuEntry } from "../components/MenuDeContexto/MenuDeContexto";
import { PainelLateral } from "../components/PainelLateral/PainelLateral";
import { useT } from "../i18n";
import { STATUS_IDS } from "./uiState";
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
  const t = useT();
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
            label: t("library.no_status"),
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
            label: t(`library.status.${status}`),
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
              label: t("library.add_collection"),
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
              label: t("library.remove_collection"),
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
      <div className={styles["actions"]} role="toolbar" aria-label={t("library.selection_actions")} aria-busy={busy}>
        <button
          disabled={busy || !collections.some((c) => !c.criteria)}
          title={t("desktop.group.collection")}
          aria-label={t("desktop.group.collection")}
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
          title={t("desktop.info.tags")}
          aria-label={t("desktop.info.tags")}
          aria-haspopup="dialog"
          onClick={() => {
            setTagsOpen(true);
          }}
        >
          <Icon name="list" />
        </button>
        <button
          disabled={busy}
          title={t("desktop.group.status")}
          aria-label={t("desktop.group.status")}
          aria-haspopup="menu"
          onClick={(e) => {
            setMenu({ anchor: e.currentTarget, kind: "status" });
          }}
        >
          <Icon name="check" />
        </button>
        <button
          disabled={busy}
          title={allFavorite ? t("library.unfavorite") : t("hint.favorite")}
          aria-label={allFavorite ? t("library.unfavorite") : t("hint.favorite")}
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
        <button title={t("common.close")} aria-label={t("common.close")} onClick={clear}>
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
        title={menu?.kind === "status" ? t("desktop.group.status") : t("desktop.group.collection")}
      />
      <PainelLateral
        open={tagsOpen}
        onClose={() => {
          setTagsOpen(false);
        }}
        title={t("desktop.info.tags")}
        closeLabel={t("common.close")}
      >
        <div className={styles["fields"]}>
          <input
            className={styles["tagInput"]}
            aria-label={t("desktop.info.tags")}
            placeholder={t("library.tag_placeholder")}
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
              {t("library.remove_tags")}
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
              {t("library.add_tags")}
            </button>
          </div>
        </div>
      </PainelLateral>
    </>
  );
}
