import { useState } from "react";
import type { Aba } from "../Abas/Abas";
import { PerfilColecoes, type CollectionItem } from "./PerfilColecoes";

export const title = "PerfilColecoes";
export const preview = "PerfilColecoes";

// Capas de demonstração em SVG leve
const CAPA_A = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 300" width="300" height="300">
  <rect width="300" height="300" fill="#2d1c24"/>
  <circle cx="150" cy="150" r="80" fill="#ff7b7b" opacity="0.4"/>
</svg>
`)}`;

const CAPA_B = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 300" width="300" height="300">
  <rect width="300" height="300" fill="#14213d"/>
  <polygon points="150,60 230,220 70,220" fill="#4d9bff" opacity="0.4"/>
</svg>
`)}`;

const CAPA_C = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 300" width="300" height="300">
  <rect width="300" height="300" fill="#1d2d1f"/>
  <circle cx="150" cy="150" r="70" fill="#3ddc84" opacity="0.4"/>
</svg>
`)}`;

const CAPA_D = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 300" width="300" height="300">
  <rect width="300" height="300" fill="#38291a"/>
  <rect x="70" y="70" width="160" height="160" rx="20" fill="#ffcf4d" opacity="0.4"/>
</svg>
`)}`;

const ABAS_ORDEM: readonly Aba<"recentes" | "az" | "mais">[] = [
  { id: "recentes", label: "Recentes" },
  { id: "az", label: "A–Z" },
  { id: "mais", label: "Mais jogos" },
];

const COLECOES_PADRAO: readonly CollectionItem[] = [
  {
    id: "c1",
    name: "Roguelikes",
    detail: "6 jogos · 4 instalados",
    covers: [CAPA_A, CAPA_B, CAPA_C, CAPA_D],
  },
  {
    id: "c2",
    name: "Jogados este mês",
    detail: "5 jogos · 5 instalados",
    automatic: true,
    covers: [CAPA_B, CAPA_C, CAPA_D, CAPA_A],
  },
  {
    id: "c3",
    name: "RPG & Aventura",
    detail: "12 jogos · 8 instalados",
    covers: [CAPA_C, CAPA_D, CAPA_A, CAPA_B],
  },
  {
    id: "c4",
    name: "Favoritos",
    detail: "3 jogos · 3 instalados",
    automatic: true,
    covers: [CAPA_D, CAPA_A, CAPA_B],
  },
];

export function Demo() {
  const [ordem, setOrdem] = useState<"recentes" | "az" | "mais">("recentes");
  const [mensagemAcao, setMensagemAcao] = useState<string | null>(null);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-32)", width: "100%", maxWidth: 1040 }}>
      {/* 1. Estado padrão com dados fiéis ao preview.html */}
      <section style={{ display: "flex", flexDirection: "column", gap: "var(--space-12)" }}>
        <h3 className="overline" style={{ margin: 0, color: "var(--text-muted)" }}>
          GRADE DE COLEÇÕES COM MOSAICO E SELO NAS AUTOMÁTICAS
        </h3>
        {mensagemAcao && (
          <div
            style={{
              padding: "var(--space-12) var(--space-16)",
              borderRadius: "var(--radius-control)",
              background: "var(--surface-raised)",
              border: "1px solid var(--accent)",
              color: "var(--text)",
              fontSize: 13,
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
            }}
          >
            <span>{mensagemAcao}</span>
            <button
              type="button"
              onClick={() => {
                setMensagemAcao(null);
              }}
              style={{
                background: "transparent",
                border: "none",
                color: "var(--text-muted)",
                cursor: "pointer",
                textDecoration: "underline",
              }}
            >
              Fechar
            </button>
          </div>
        )}
        <PerfilColecoes
          summary="4 coleções · 26 jogos"
          autoBadgeLabel="Automática"
          order={{
            label: "Ordem das coleções",
            tabs: ABAS_ORDEM,
            value: ordem,
            onChange: setOrdem,
          }}
          collections={COLECOES_PADRAO}
          newCollection={{
            label: "Nova coleção",
            onClick: () => {
              setMensagemAcao("Ação disparada: Nova coleção");
            },
          }}
          importCollection={{
            label: "Importar coleção",
            onClick: () => {
              setMensagemAcao("Ação disparada: Importar coleção");
            },
          }}
          onCollectionClick={(c) => {
            setMensagemAcao(`Coleção aberta: ${c.name}`);
          }}
        />
      </section>

      {/* 2. Estado vazio */}
      <section style={{ display: "flex", flexDirection: "column", gap: "var(--space-12)" }}>
        <h3 className="overline" style={{ margin: 0, color: "var(--text-muted)" }}>
          ESTADO VAZIO (SEM COLEÇÕES)
        </h3>
        <PerfilColecoes
          summary="0 coleções · 0 jogos"
          collections={[]}
          emptyMessage="Nenhuma coleção criada ainda. Use os botões abaixo para criar ou importar."
          newCollection={{
            label: "Nova coleção",
            onClick: () => undefined,
          }}
          importCollection={{
            label: "Importar coleção",
            onClick: () => undefined,
          }}
        />
      </section>

      {/* 3. Sem dado (bloco some) */}
      <section style={{ display: "flex", flexDirection: "column", gap: "var(--space-12)" }}>
        <h3 className="overline" style={{ margin: 0, color: "var(--text-muted)" }}>
          SEM DADOS (O BLOCO SOME)
        </h3>
        <PerfilColecoes />
        <p style={{ margin: 0, fontSize: 13, color: "var(--text-dim)" }}>
          (O componente recebeu props vazias e retornou null com sucesso)
        </p>
      </section>
    </div>
  );
}
