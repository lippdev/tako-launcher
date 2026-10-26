import { useMemo, useState } from "react";
import { call } from "../../bridge";
import { Abas, type Aba } from "../../components/Abas/Abas";
import { Aviso } from "../../components/Aviso/Aviso";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { useT } from "../../i18n";
import { ColecoesDoPerfil } from "./ColecoesDoPerfil";
import { PerfisDialogo } from "./PerfisDialogo";
import { PROFILE_TABS, initial, lastPlayedText, profileName, type ProfileTab } from "./perfilModel";
import { SessoesDoPerfil } from "./SessoesDoPerfil";
import { useBridgeData } from "./useBridgeData";
import { useProfiles } from "./useProfiles";
import { VisaoGeralDoPerfil } from "./VisaoGeralDoPerfil";
import styles from "./TelaPerfil.module.css";

interface TelaPerfilProps {
  /** Abrir a página do jogo na biblioteca. */
  onOpenGame: (id: string) => void;
  /** Abrir a biblioteca já filtrada numa coleção. */
  onOpenCollection: (id: string) => void;
}

const loadSummary = () => call("profile.summary");
const TAB_PREFIX = "perfil";

/**
 * A tela Perfil (README da TelaPerfil, paridade com `tako-ui/src/shell/profile.rs`): a identidade do perfil em uso, as
 * guias Visão geral · Sessões · Coleções e, no cabeçalho, Trocar perfil e Editar perfil (UX-004). Tudo vem do
 * núcleo; a tela só guarda a guia aberta e os diálogos. A guia Família e os blocos de conquistas ficam de fora até
 * existir o que os alimenta, como no app Slint.
 */
export function TelaPerfil({ onOpenGame, onOpenCollection }: TelaPerfilProps) {
  const t = useT();
  const { data: summary, status, reload } = useBridgeData(loadSummary);
  const [tab, setTab] = useState<ProfileTab>("overview");
  const [dialog, setDialog] = useState<{ manageId: string | null } | null>(null);
  const [notice, setNotice] = useState({ open: false, text: "", key: 0 });
  const profiles = useProfiles(dialog !== null);

  const tabs: readonly Aba<ProfileTab>[] = useMemo(
    () => PROFILE_TABS.map((id) => ({ id, label: t(`profile.tab_${id}`) })),
    [t],
  );

  const name = useMemo(() => (summary ? profileName(summary.profile, t) : ""), [summary, t]);
  const last = useMemo(
    () => (summary ? lastPlayedText(summary.lastPlayedAt, summary.today, t) : null),
    [summary, t],
  );
  const line = useMemo(
    () =>
      summary
        ? [summary.profile.main ? t("profile.main") : null, last].filter((text): text is string => text !== null)
        : [],
    [summary, last, t],
  );

  if (summary === null) {
    return (
      <div className={styles["pagina"]}>
        {status === "error" ? (
          <div className={styles["aviso"]} role="alert">
            {t("profile.web.load_error")}
            <button type="button" className={styles["retry"]} onClick={reload}>
              {t("profile.web.retry")}
            </button>
          </div>
        ) : (
          <p className={styles["aviso"]} role="status">
            {t("profile.web.loading")}
          </p>
        )}
      </div>
    );
  }

  return (
    <div className={styles["pagina"]}>
      <header className={styles["cabeca"]}>
        <span className={styles["avatar"]} aria-hidden="true">
          {initial(name)}
        </span>
        <div className={styles["identidade"]}>
          <h1 className={styles["nome"]}>{name}</h1>
          {line.length > 0 && (
            <p className={styles["linha"]}>
              {line.map((text) => (
                <span key={text}>{text}</span>
              ))}
            </p>
          )}
        </div>
        <div className={styles["acoes"]}>
          <BotaoDesktop
            variant="outlined"
            icon="users"
            onClick={() => {
              setDialog({ manageId: null });
            }}
          >
            {t("profile.switch")}
          </BotaoDesktop>
          <BotaoDesktop
            variant="outlined"
            icon="user"
            onClick={() => {
              setDialog({ manageId: summary.profile.id });
            }}
          >
            {t("profile.edit")}
          </BotaoDesktop>
        </div>
      </header>

      <Abas variant="loja" label={t("profile.web.tabs")} idPrefix={TAB_PREFIX} tabs={tabs} value={tab} onChange={setTab} />

      <div
        className={styles["painel"]}
        role="tabpanel"
        id={`${TAB_PREFIX}-panel-${tab}`}
        aria-labelledby={`${TAB_PREFIX}-tab-${tab}`}
      >
        {tab === "overview" && (
          <VisaoGeralDoPerfil
            summary={summary}
            onOpenGame={onOpenGame}
            onSeeAll={() => {
              setTab("sessions");
            }}
          />
        )}
        {tab === "sessions" && <SessoesDoPerfil onOpenGame={onOpenGame} />}
        {tab === "collections" && <ColecoesDoPerfil onOpenCollection={onOpenCollection} />}
      </div>

      <PerfisDialogo
        open={dialog !== null}
        onClose={() => {
          setDialog(null);
        }}
        manageId={dialog?.manageId ?? null}
        profiles={profiles}
        onChanged={reload}
        onSwitched={(switched) => {
          setNotice((before) => ({ open: true, text: t("profiles.web.restart_notice", { name: switched }), key: before.key + 1 }));
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
    </div>
  );
}
