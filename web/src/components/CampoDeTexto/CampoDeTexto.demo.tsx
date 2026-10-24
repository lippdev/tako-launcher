import { useState } from "react";
import { CampoDeTexto } from "./CampoDeTexto";
import { DemoGrupo } from "../../lib/demoGrupo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { Dialogo } from "../Dialogo/Dialogo";

export const title = "CampoDeTexto";
export const preview = "CampoDeTexto";

const noop = () => undefined;
const coluna = { width: 360 } as const;

export function Demo() {
  const [interativo, setInterativo] = useState("Roguelikes");
  const [comErro, setComErro] = useState("Favoritos");
  const [dialogoAberto, setDialogoAberto] = useState(false);
  const [nomeColecao, setNomeColecao] = useState("Roguelikes");

  return (
    <div>
      <DemoGrupo titulo="Estados do campo de texto — interativo (digite no campo) · foco (estático) · com erro · desabilitado · numérico">
        <div style={coluna}>
          <CampoDeTexto
            label="Nome da coleção (interativo)"
            value={interativo}
            onChange={setInterativo}
            help="Até 40 caracteres, sem repetir o nome de outra coleção."
            maxLength={40}
            showCount
          />
        </div>

        <div style={coluna}>
          <CampoDeTexto
            label="Nome da coleção (foco)"
            value="Roguelikes"
            onChange={noop}
            help="Borda clara e leve brilho no tema TAKO"
            data-force="focus"
          />
        </div>

        <div style={coluna}>
          <CampoDeTexto
            label="Nome da coleção (com erro)"
            value={comErro}
            onChange={setComErro}
            error="Já existe uma coleção com este nome."
            maxLength={40}
            showCount
          />
        </div>

        <div style={coluna}>
          <CampoDeTexto
            label="Chave de API (desabilitado)"
            value="tk_live_94820381029481029"
            onChange={noop}
            help="Gerada automaticamente pelo sistema."
            disabled
          />
        </div>

        <div style={coluna}>
          <CampoDeTexto
            label="Limite de taxa de quadros (numérico)"
            type="number"
            value="144"
            onChange={noop}
            help="Defina 0 para taxa ilimitada."
          />
        </div>
      </DemoGrupo>

      <DemoGrupo titulo="Diálogo da prévia (README do CampoDeTexto e preview.html a 1600×900)">
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setDialogoAberto(true);
          }}
        >
          Abrir diálogo 'Nova coleção' (tamanho S)
        </BotaoDesktop>
      </DemoGrupo>

      <Dialogo
        open={dialogoAberto}
        size="s"
        title="Nova coleção"
        closeLabel="Fechar (Esc)"
        onClose={() => {
          setDialogoAberto(false);
        }}
        actions={
          <>
            <BotaoDesktop
              variant="outlined"
              onClick={() => {
                setDialogoAberto(false);
              }}
            >
              Cancelar
            </BotaoDesktop>
            <BotaoDesktop
              variant="primary"
              size="sm"
              icon="check"
              onClick={() => {
                setDialogoAberto(false);
              }}
            >
              Criar
            </BotaoDesktop>
          </>
        }
      >
        <CampoDeTexto
          label="Nome da coleção"
          hideLabel
          value={nomeColecao}
          onChange={setNomeColecao}
          help="Até 40 caracteres, sem repetir o nome de outra coleção."
          maxLength={40}
          autoFocus
        />
      </Dialogo>
    </div>
  );
}
