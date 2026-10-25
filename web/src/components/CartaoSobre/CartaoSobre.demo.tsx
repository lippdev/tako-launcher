import { DemoGrupo } from "../../lib/demoGrupo";
import { CartaoSobre } from "./CartaoSobre";

export const title = "CartaoSobre";
export const preview = "CartaoSobre";

const TEXTO = [
  "Da Santa Monica Studio, trazido ao PC com a Jetpack Interactive, é uma jornada épica e emocionante de Kratos e Atreus entre segurar e deixar ir.",
  "Continuação do aclamado capítulo anterior, a história começa com o Fimbulwinter já avançado. Kratos e Atreus percorrem os Nove Reinos atrás de respostas enquanto as forças de Odin se preparam para a batalha profetizada.",
  "No caminho, exploram paisagens míticas e enfrentam deuses e monstros nórdicos.",
];

export function Demo() {
  return (
    <DemoGrupo titulo="Texto da página do jogo (644 de largura e 320); sem texto, o cartão não existe">
      <div style={{ width: 644 }}>
        <CartaoSobre title="Sobre o jogo" paragrafos={TEXTO} />
      </div>
      <div style={{ width: 320 }}>
        <CartaoSobre title="Sobre o jogo" paragrafos={TEXTO.slice(2)} />
      </div>
      <CartaoSobre title="Sobre o jogo" paragrafos={[]} />
    </DemoGrupo>
  );
}
