import { Icon } from "../Icon/Icon";
import { LojaEncontrada, LojaEncontradaGroup } from "./LojaEncontrada";

export const title = "LojaEncontrada";
export const preview = "LojaEncontrada";

export function Demo() {
  const storeIcon = <Icon name="store" size="sm" />;
  const folderIcon = <Icon name="folder" size="sm" />;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "28px", padding: "24px", maxWidth: "600px" }}>
      <section>
        <h3 style={{ margin: "0 0 16px", color: "var(--text-muted)", fontSize: "14px" }}>
          Lojas no computador
        </h3>
        <LojaEncontradaGroup>
          <LojaEncontrada
            icon={storeIcon}
            name="Steam"
            pathOrDetails="C:\Program Files (x86)\Steam"
            status="ready"
            statusLabel="18 jogos"
          />
          <LojaEncontrada
            icon={storeIcon}
            name="Epic Games"
            pathOrDetails="C:\Program Files\Epic Games"
            status="ready"
            statusLabel="4 jogos"
          />
          <LojaEncontrada
            icon={storeIcon}
            name="GOG Galaxy"
            pathOrDetails="D:\GOG Games"
            status="ready"
            statusLabel="1 jogo"
          />
          <LojaEncontrada
            icon={storeIcon}
            name="Xbox"
            pathOrDetails="Microsoft Store"
            status="not-installed"
            statusLabel="Não instalado"
          />
        </LojaEncontradaGroup>
      </section>

      <section>
        <h3 style={{ margin: "0 0 16px", color: "var(--text-muted)", fontSize: "14px" }}>
          Emuladores e pastas
        </h3>
        <LojaEncontradaGroup>
          <LojaEncontrada
            icon={storeIcon}
            name="RetroArch"
            pathOrDetails="SNES, Mega Drive, PS1 · 212 jogos"
            status="ready"
            statusLabel="212 jogos"
          />
          <LojaEncontrada
            icon={storeIcon}
            name="Dolphin"
            pathOrDetails="GameCube, Wii · 14 jogos"
            status="ready"
            statusLabel="14 jogos"
          />
          <LojaEncontrada
            icon={storeIcon}
            name="PCSX2"
            pathOrDetails="PlayStation 2"
            status="missing"
            statusLabel="falta a BIOS"
            actionLabel="Escolher BIOS…"
            onAction={() => alert("Escolher BIOS")}
          />
          <LojaEncontrada
            icon={storeIcon}
            name="DuckStation"
            pathOrDetails="PlayStation"
            status="not-installed"
            statusLabel="Não instalado"
          />
          <LojaEncontrada
            icon={folderIcon}
            name="Pasta de ROMs"
            pathOrDetails="D:\ROMs · 228 jogos em 6 sistemas"
            status="action"
            actionLabel="Trocar"
            onAction={() => alert("Trocar pasta")}
          />
        </LojaEncontradaGroup>
      </section>
    </div>
  );
}
