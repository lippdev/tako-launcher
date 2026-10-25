import { useState } from "react";
import type { OpcaoDeSelecao } from "../CampoDeSelecao/CampoDeSelecao";
import { PerfilFamilia, type PerfilFamiliaItem } from "./PerfilFamilia";

export const title = "PerfilFamilia";
export const preview = "PerfilFamilia";

const OPCOES_TEMPO: readonly OpcaoDeSelecao<string>[] = [
  { value: "sem-limite", label: "Sem limite" },
  { value: "1h", label: "1 hora" },
  { value: "2h", label: "2 horas" },
  { value: "3h", label: "3 horas" },
];

const OPCOES_HORARIO: readonly OpcaoDeSelecao<string>[] = [
  { value: "livre", label: "Livre a qualquer hora" },
  { value: "08-21", label: "08:00 – 21:00" },
  { value: "09-20", label: "09:00 – 20:00" },
];

const OPCOES_CLASSIFICACAO: readonly OpcaoDeSelecao<string>[] = [
  { value: "livre", label: "Livre para todas as idades" },
  { value: "10", label: "Até 10 anos" },
  { value: "12", label: "Até 12 anos" },
  { value: "14", label: "Até 14 anos" },
  { value: "16", label: "Até 16 anos" },
  { value: "18", label: "Sem restrição (18+)" },
];

const OPCOES_JOGOS_ACIMA: readonly OpcaoDeSelecao<string>[] = [
  { value: "cadeado", label: "Com cadeado (pede o PIN)" },
  { value: "esconder", label: "Esconder da biblioteca" },
  { value: "permitir", label: "Permitir todos os jogos" },
];

const PERFIS_INICIAIS: readonly PerfilFamiliaItem[] = [
  { id: "p1", name: "alyx", detail: "Responsável · sem limites", initial: "A" },
  { id: "p2", name: "Bia", detail: "Até 12 anos · 2 h por dia", initial: "B", active: true },
  { id: "p3", name: "Convidado", detail: "Não guarda sessões nem conquistas", initial: "C" },
];

export function Demo() {
  const [perfis, setPerfis] = useState<readonly PerfilFamiliaItem[]>(PERFIS_INICIAIS);
  const [tempo, setTempo] = useState("2h");
  const [horario, setHorario] = useState("08-21");
  const [classificacao, setClassificacao] = useState("12");
  const [pinAtivo, setPinAtivo] = useState(true);
  const [jogosAcima, setJogosAcima] = useState("cadeado");
  const [aviso, setAviso] = useState<string | null>(null);

  const perfilAtivo = perfis.find((p) => p.active) ?? perfis[1];

  const handleSelecionarPerfil = (selecionado: PerfilFamiliaItem) => {
    setPerfis(
      perfis.map((p) => ({
        ...p,
        active: p.id === selecionado.id,
      })),
    );
    setAviso(`Perfil selecionado: ${selecionado.name}`);
  };

  const isResponsavel = perfilAtivo?.id === "p1";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-32)", width: "100%", maxWidth: 1040 }}>
      {/* 1. Demonstração interativa completa fiel ao protótipo */}
      <section style={{ display: "flex", flexDirection: "column", gap: "var(--space-12)" }}>
        <h3 className="overline" style={{ margin: 0, color: "var(--text-muted)" }}>
          CONTROLES PARENTAIS E LIMITES POR PERFIL (EXEMPLO COMPLETO)
        </h3>
        {aviso && (
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
            <span>{aviso}</span>
            <button
              type="button"
              onClick={() => {
                setAviso(null);
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
        <PerfilFamilia
          profilesTitle="Perfis neste PC"
          profiles={perfis}
          onSelectProfile={handleSelecionarPerfil}
          addProfile={{
            label: "Adicionar perfil",
            onClick: () => {
              setAviso("Ação disparada: Adicionar perfil");
            },
          }}
          {...(!isResponsavel
            ? {
                today: {
                  title: `Hoje · ${perfilAtivo?.name ?? "Bia"}`,
                  timeUsed: "1 h 12 min de 2 h",
                  timeRemaining: "Sobram 48 min até as 21:00",
                  progress: 0.6,
                },
                limits: {
                  title: "Limites",
                  timePerDay: {
                    label: "Tempo por dia",
                    description: "Avisa 10 minutos antes de acabar",
                    value: tempo,
                    options: OPCOES_TEMPO,
                    onChange: setTempo,
                  },
                  schedule: {
                    label: "Horário permitido",
                    description: "Fora dele, o perfil não abre jogos",
                    value: horario,
                    options: OPCOES_HORARIO,
                    onChange: setHorario,
                  },
                  rating: {
                    label: "Classificação indicativa",
                    description: "Jogos acima ficam escondidos ou com cadeado",
                    value: classificacao,
                    options: OPCOES_CLASSIFICACAO,
                    onChange: setClassificacao,
                  },
                },
              }
            : {})}
          access={{
            title: "Acesso",
            pin: {
              label: "PIN para sair do perfil",
              description: "Pedido ao trocar para outro perfil",
              checked: pinAtivo,
              onChange: setPinAtivo,
            },
            ...(!isResponsavel
              ? {
                  gamesAboveRating: {
                    label: "Jogos acima da classificação",
                    value: jogosAcima,
                    options: OPCOES_JOGOS_ACIMA,
                    onChange: setJogosAcima,
                  },
                }
              : {}),
          }}
        />
      </section>

      {/* 2. Perfil sem limites (o bloco "Hoje" e "Limites" somem) */}
      <section style={{ display: "flex", flexDirection: "column", gap: "var(--space-12)" }}>
        <h3 className="overline" style={{ margin: 0, color: "var(--text-muted)" }}>
          PERFIL SEM LIMITES CONFIGURADOS (SEÇÃO HOJE E LIMITES SOMEM)
        </h3>
        <PerfilFamilia
          profilesTitle="Perfis neste PC"
          profiles={[
            { id: "p1", name: "alyx", detail: "Responsável · sem limites", initial: "A", active: true },
            { id: "p2", name: "Bia", detail: "Até 12 anos · 2 h por dia", initial: "B" },
          ]}
          access={{
            title: "Acesso",
            pin: {
              label: "PIN para sair do perfil",
              description: "Pedido ao trocar para outro perfil",
              checked: false,
              onChange: () => undefined,
            },
          }}
        />
      </section>

      {/* 3. Sem dado (o bloco todo some) */}
      <section style={{ display: "flex", flexDirection: "column", gap: "var(--space-12)" }}>
        <h3 className="overline" style={{ margin: 0, color: "var(--text-muted)" }}>
          SEM DADOS (O BLOCO SOME)
        </h3>
        <PerfilFamilia />
        <p style={{ margin: 0, fontSize: 13, color: "var(--text-dim)" }}>
          (O componente recebeu props vazias e retornou null com sucesso)
        </p>
      </section>
    </div>
  );
}
