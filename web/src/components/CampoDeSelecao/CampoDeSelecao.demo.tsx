import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { CampoDeSelecao } from "./CampoDeSelecao";

export const title = "CampoDeSelecao";
export const preview = "CampoDeSelecao";

const MODO = [
  { value: "janela", label: "Janela" },
  { value: "tela-cheia", label: "Tela cheia" },
  { value: "sem-borda", label: "Sem bordas" },
] as const;
const IDIOMA = [
  { value: "pt-BR", label: "Português (Brasil)" },
  { value: "en", label: "English" },
  { value: "es", label: "Español" },
  { value: "fr", label: "Français" },
  { value: "de", label: "Deutsch" },
  { value: "it", label: "Italiano" },
  { value: "ja", label: "日本語" },
  { value: "ko", label: "한국어" },
  { value: "zh", label: "中文" },
  { value: "ru", label: "Русский" },
  { value: "pl", label: "Polski" },
  { value: "tr", label: "Türkçe" },
] as const;
const noop = () => undefined;

export function Demo() {
  const [modo, setModo] = useState<(typeof MODO)[number]["value"]>("janela");
  const [idioma, setIdioma] = useState<(typeof IDIOMA)[number]["value"]>("pt-BR");
  return (
    <div>
      <DemoGrupo titulo="Seletor — clique, ↓ ou ↑ abre o menu com o título da linha e ✓ na atual">
        <CampoDeSelecao label="Modo da janela" options={MODO} value={modo} onChange={setModo} />
        <CampoDeSelecao label="Idioma" options={IDIOMA} value={idioma} onChange={setIdioma} />
      </DemoGrupo>
      <DemoGrupo titulo="Estados — normal · hover · foco · desabilitado">
        <CampoDeSelecao label="Modo da janela" options={MODO} value="janela" onChange={noop} />
        <CampoDeSelecao label="Modo da janela (hover)" options={MODO} value="janela" onChange={noop} data-force="hover" />
        <CampoDeSelecao label="Modo da janela (foco)" options={MODO} value="janela" onChange={noop} data-force="focus" />
        <CampoDeSelecao label="Modo da janela (desabilitado)" options={MODO} value="janela" onChange={noop} disabled />
      </DemoGrupo>
    </div>
  );
}
