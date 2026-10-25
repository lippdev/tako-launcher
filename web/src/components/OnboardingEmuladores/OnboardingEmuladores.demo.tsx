import { TelaOnboarding } from "../TelaOnboarding/TelaOnboarding";
import {
  OnboardingEmuladores,
  type OnboardingEmulatorItem,
} from "./OnboardingEmuladores";

export const title = "OnboardingEmuladores";
export const preview = "OnboardingEmuladores";

function EmulatorIcon({ letter }: { letter: string }) {
  return (
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: "24px",
        height: "24px",
        borderRadius: "6px",
        backgroundColor: "var(--glass-control)",
        color: "var(--text)",
        fontWeight: 700,
        fontSize: "11px",
      }}
    >
      {letter}
    </span>
  );
}

export function Demo() {
  const emulators: OnboardingEmulatorItem[] = [
    {
      id: "retroarch",
      name: "RetroArch",
      details: "SNES, Mega Drive, PS1 · 212 jogos",
      icon: <EmulatorIcon letter="RA" />,
      status: "ready",
      statusLabel: "212 jogos",
    },
    {
      id: "dolphin",
      name: "Dolphin",
      details: "GameCube, Wii · 14 jogos",
      icon: <EmulatorIcon letter="DO" />,
      status: "ready",
      statusLabel: "14 jogos",
    },
    {
      id: "pcsx2",
      name: "PCSX2",
      details: "PlayStation 2",
      icon: <EmulatorIcon letter="P2" />,
      status: "missing",
      statusLabel: "falta a BIOS",
      actionLabel: "Escolher BIOS…",
      onAction: () => alert("Escolher BIOS"),
    },
    {
      id: "duckstation",
      name: "DuckStation",
      details: "PlayStation",
      icon: <EmulatorIcon letter="DU" />,
      status: "not-installed",
      statusLabel: "Não instalado",
    },
  ];

  const romsFolder = {
    name: "Pasta de ROMs",
    details: "D:\\ROMs · 228 jogos em 6 sistemas",
    actionLabel: "Trocar",
    onAction: () => alert("Trocar pasta"),
  };

  return (
    <div style={{ width: "1600px", height: "900px", position: "relative" }}>
      <TelaOnboarding
        skipLabel="Pular configuração"
        onSkip={() => alert("Pular")}
      >
        <OnboardingEmuladores
          stepLabel="2 de 5"
          stepAriaLabel="Passo 2 de 5"
          title="Jogos de emuladores."
          description="O TAKO acha emuladores e ROMs e abre cada jogo no emulador certo."
          emulators={emulators}
          romsFolder={romsFolder}
          continueLabel="Continuar"
          onContinue={() => alert("Continuar")}
          addEmulatorLabel="Adicionar emulador…"
          onAddEmulator={() => alert("Adicionar")}
          skipEmulatorsLabel="Não uso emuladores"
          onSkipEmulators={() => alert("Não uso")}
          footerNote="O TAKO não baixa ROMs nem BIOS."
        />
      </TelaOnboarding>
    </div>
  );
}
