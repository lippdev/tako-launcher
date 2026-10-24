import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { Icon } from "../Icon/Icon";
import { LojaEncontrada, LojaEncontradaGroup } from "../LojaEncontrada/LojaEncontrada";
import { PassosDoOnboarding } from "../PassosDoOnboarding/PassosDoOnboarding";
import { TelaOnboarding } from "./TelaOnboarding";

export const title = "TelaOnboarding";
export const preview = "TelaOnboarding";

export function Demo() {
  const storeIcon = <Icon name="store" size="sm" />;

  return (
    <div style={{ width: "1600px", height: "900px", position: "relative" }}>
      <TelaOnboarding
        skipLabel="Pular configuração"
        onSkip={() => alert("Pular configuração")}
      >
        <PassosDoOnboarding
          currentStep={1}
          totalSteps={5}
          label="1 de 5"
          ariaLabel="Passo 1 de 5"
        />

        <h1
          style={{
            fontSize: "40px",
            fontWeight: 800,
            letterSpacing: "-0.025em",
            lineHeight: 1.1,
            margin: 0,
            color: "var(--text)",
          }}
        >
          Achamos seus jogos.
        </h1>

        <p
          style={{
            fontSize: "15px",
            lineHeight: 1.55,
            color: "var(--text-muted)",
            maxWidth: "520px",
            margin: 0,
          }}
        >
          Estas lojas estão instaladas neste PC. O TAKO lê a biblioteca de cada uma, sem pedir senha.
        </p>

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

        <div style={{ display: "flex", gap: "10px", alignItems: "center", marginTop: "8px" }}>
          <BotaoDesktop variant="primary" icon="chevron-right">
            Continuar
          </BotaoDesktop>
          <BotaoDesktop variant="outlined" icon="folder">
            Adicionar pasta…
          </BotaoDesktop>
        </div>

        <p
          style={{
            fontSize: "12.5px",
            color: "var(--text-dim)",
            marginTop: "4px",
            margin: 0,
          }}
        >
          Dá para mudar tudo depois em Configurações › Biblioteca e lojas.
        </p>
      </TelaOnboarding>
    </div>
  );
}
