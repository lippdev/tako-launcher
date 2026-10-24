import { useState } from "react";
import { Aviso } from "../../components/Aviso/Aviso";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { profileStrings as s } from "../strings";
import { PerfisDialogo } from "./PerfisDialogo";
import { profileName } from "./perfilModel";
import { useProfiles } from "./useProfiles";

/**
 * Configurações › Geral › Perfil (UX-004): a linha com o perfil em uso, que abre a lista para criar, renomear, apagar,
 * trocar e escolher se o Tako pergunta quem está jogando ao abrir.
 */
export function PerfisNasConfiguracoes() {
  const t = useT();
  const profiles = useProfiles();
  const [open, setOpen] = useState(false);
  const [notice, setNotice] = useState({ open: false, text: "", key: 0 });

  const current = profiles.list?.profiles.find((profile) => profile.inUse);

  return (
    <>
      <GrupoDeConfiguracao title={t("profiles.title")}>
        <LinhaDeConfiguracao
          title={t("profiles.title")}
          description={t("profiles.desc")}
          {...(current ? { value: profileName(current, t) } : {})}
          onClick={() => {
            setOpen(true);
          }}
        />
      </GrupoDeConfiguracao>
      <PerfisDialogo
        open={open}
        onClose={() => {
          setOpen(false);
        }}
        manageId={null}
        profiles={profiles}
        onChanged={profiles.reload}
        onSwitched={(name) => {
          setNotice((before) => ({ open: true, text: s.restartNotice(name), key: before.key + 1 }));
        }}
      />
      <Aviso
        open={notice.open}
        tone="info"
        noticeKey={notice.key}
        duration={5000}
        onClose={() => {
          setNotice((before) => ({ ...before, open: false }));
        }}
      >
        {notice.text}
      </Aviso>
    </>
  );
}
