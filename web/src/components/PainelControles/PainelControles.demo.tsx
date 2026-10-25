import { useState } from "react";
import { PainelControles } from "./PainelControles";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { DemoGrupo } from "../../lib/demoGrupo";
import type { OpcaoDeSelecao } from "../CampoDeSelecao/CampoDeSelecao";

export const title = "PainelControles";
export const preview = "TelaControles";

const OPCOES_ESTILO: readonly OpcaoDeSelecao<string>[] = [
  { value: "xbox", label: "Xbox" },
  { value: "playstation", label: "PlayStation" },
  { value: "nintendo", label: "Nintendo" },
];

export function Demo() {
  const [conectado, setConectado] = useState(true);
  const [estiloBotao, setEstiloBotao] = useState("xbox");
  const [vibracao, setVibracao] = useState(true);
  const [zonaMorta, setZonaMorta] = useState(8);
  const [navegacao, setNavegacao] = useState(true);
  const [mensagemTeste, setMensagemTeste] = useState<string | null>(null);

  const formatarPorcentagem = (valor: number) => `${valor}%`;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-24)", maxWidth: 760 }}>
      <DemoGrupo titulo="Estados de controle e estilos de botões">
        <BotaoDesktop
          variant={conectado ? "primary" : "outlined"}
          size="sm"
          onClick={() => {
            setConectado(true);
            setMensagemTeste(null);
          }}
        >
          Controle conectado (Xbox · 85% bateria)
        </BotaoDesktop>
        <BotaoDesktop
          variant={!conectado ? "primary" : "outlined"}
          size="sm"
          onClick={() => {
            setConectado(false);
            setMensagemTeste(null);
          }}
        >
          Controle desconectado
        </BotaoDesktop>
        <BotaoDesktop
          variant={estiloBotao === "xbox" ? "primary" : "outlined"}
          size="sm"
          onClick={() => {
            setEstiloBotao("xbox");
          }}
        >
          Botões: Xbox
        </BotaoDesktop>
        <BotaoDesktop
          variant={estiloBotao === "playstation" ? "primary" : "outlined"}
          size="sm"
          onClick={() => {
            setEstiloBotao("playstation");
          }}
        >
          Botões: PlayStation
        </BotaoDesktop>
        <BotaoDesktop
          variant={estiloBotao === "nintendo" ? "primary" : "outlined"}
          size="sm"
          onClick={() => {
            setEstiloBotao("nintendo");
          }}
        >
          Botões: Nintendo
        </BotaoDesktop>
      </DemoGrupo>

      {mensagemTeste && (
        <p className="label-sm" style={{ margin: 0, color: "var(--text-muted)" }}>
          Resultado: <strong>{mensagemTeste}</strong>
        </p>
      )}

      <PainelControles
        title="Controles"
        description="Botões, vibração e navegação."
        sectionTitle="Controle"
        testTitle="Testar controle"
        controllerDetails={
          conectado
            ? "Controle sem fio Xbox · 85% · conectado"
            : "Nenhum controle conectado"
        }
        testActionLabel="Testar"
        testDisabled={!conectado}
        onTest={() => {
          setMensagemTeste("Teste do controle disparado com sucesso.");
        }}
        buttonStyleTitle="Botões"
        buttonStyleDescription="Ícones nas dicas e menus"
        buttonStyle={estiloBotao}
        buttonStyleOptions={OPCOES_ESTILO}
        onButtonStyleChange={(novoEstilo) => {
          setEstiloBotao(novoEstilo);
        }}
        buttonStyleAriaLabel="Estilo dos botões"
        vibrationTitle="Vibração"
        vibrationDescription="Vibra ao navegar na tela cheia"
        vibration={vibracao}
        onVibrationChange={setVibracao}
        vibrationAriaLabel="Vibração ao navegar"
        deadzoneTitle="Zona morta do analógico"
        deadzoneDescription="Ignora movimentos pequenos do analógico"
        deadzone={zonaMorta}
        onDeadzoneChange={setZonaMorta}
        deadzoneMin={0}
        deadzoneMax={30}
        deadzoneStep={1}
        deadzoneFormat={formatarPorcentagem}
        deadzoneAriaLabel="Zona morta do analógico"
        navigationTitle="Navegar no TAKO com controle"
        navigationDescription="Direcional e analógicos na interface do desktop"
        navigation={navegacao}
        onNavigationChange={setNavegacao}
        navigationAriaLabel="Navegar no TAKO com controle"
      />
    </div>
  );
}
