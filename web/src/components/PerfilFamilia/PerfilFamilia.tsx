import type { CSSProperties, KeyboardEvent } from "react";
import { cx } from "../../lib/cx";
import { CampoDeSelecao, type OpcaoDeSelecao } from "../CampoDeSelecao/CampoDeSelecao";
import { Chave } from "../Chave/Chave";
import { Icon } from "../Icon/Icon";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../LinhaDeConfiguracao/LinhaDeConfiguracao";
import styles from "./PerfilFamilia.module.css";

export interface PerfilFamiliaItem {
  id: string;
  /** Nome do perfil, ex.: "alyx", "Bia", "Convidado". */
  name: string;
  /** Descrição dos limites, ex.: "Responsável · sem limites" ou "Até 12 anos · 2 h por dia". */
  detail?: string | undefined;
  /** Inicial exibida no avatar (se omitida, usa a primeira letra do nome). */
  initial?: string | undefined;
  /** Indica se é o perfil selecionado atualmente na tela. */
  active?: boolean | undefined;
}

export interface LimiteCampoConfig<T extends string = string> {
  label: string;
  description?: string | undefined;
  value: T;
  options: readonly OpcaoDeSelecao<T>[];
  onChange: (value: T) => void;
}

export interface ChaveCampoConfig {
  label: string;
  description?: string | undefined;
  checked: boolean;
  onChange: (checked: boolean) => void;
}

export interface BlocoHoje {
  /** Título da seção, ex.: "Hoje · Bia". */
  title: string;
  /** Tempo usado em destaque, ex.: "1 h 12 min de 2 h". */
  timeUsed: string;
  /** Tempo restante ou horário limite, ex.: "Sobram 48 min até as 21:00". */
  timeRemaining?: string | undefined;
  /** Fração de 0 a 1 para preenchimento da barra de progresso. */
  progress: number;
  /** Rótulo acessível da barra para leitores de tela. */
  ariaLabel?: string | undefined;
}

export interface BlocoLimites {
  /** Título do grupo, ex.: "Limites". */
  title: string;
  timePerDay?: LimiteCampoConfig | undefined;
  schedule?: LimiteCampoConfig | undefined;
  rating?: LimiteCampoConfig | undefined;
}

export interface BlocoAcesso {
  /** Título do grupo, ex.: "Acesso". */
  title: string;
  pin?: ChaveCampoConfig | undefined;
  gamesAboveRating?: LimiteCampoConfig | undefined;
}

export interface PerfilFamiliaProps {
  /** Título do painel de perfis à esquerda, ex.: "Perfis neste PC". */
  profilesTitle?: string | undefined;
  /** Lista de perfis do computador. */
  profiles?: readonly PerfilFamiliaItem[] | undefined;
  /** Callback ao selecionar um perfil. */
  onSelectProfile?: ((perfil: PerfilFamiliaItem) => void) | undefined;
  /** Botão para adicionar novo perfil na lista à esquerda. */
  addProfile?:
    | {
        label: string;
        onClick: () => void;
      }
    | undefined;
  /** Bloco "Hoje" com barra de progresso do tempo usado no dia. */
  today?: BlocoHoje | undefined;
  /** Bloco "Limites" (tempo por dia, horário, classificação). */
  limits?: BlocoLimites | undefined;
  /** Bloco "Acesso" (PIN para sair do perfil, jogos acima da classificação). */
  access?: BlocoAcesso | undefined;
  className?: string | undefined;
}

/**
 * Componente PerfilFamilia (README do PerfilFamilia):
 * Exibe à esquerda os perfis do PC (com o ativo selecionado) e à direita os grupos de
 * configuração da família: Hoje (progresso), Limites e Acesso.
 * Sem dados, cada bloco correspondente desaparece.
 */
export function PerfilFamilia({
  profilesTitle,
  profiles,
  onSelectProfile,
  addProfile,
  today,
  limits,
  access,
  className,
}: PerfilFamiliaProps) {
  const temColunaPerfis = Boolean((profiles && profiles.length > 0) || addProfile);
  const temColunaControles = Boolean(today || limits || access);

  // Sem nenhum dado, o componente não renderiza
  if (!temColunaPerfis && !temColunaControles) {
    return null;
  }

  const progressoClamped = today ? Math.min(1, Math.max(0, today.progress)) : 0;

  return (
    <div className={cx(styles["container"], className)}>
      {/* Coluna esquerda: Lista de perfis do computador */}
      {temColunaPerfis && (
        <aside className={styles["colunaPerfis"]} aria-label={profilesTitle ?? "Perfis"}>
          {profilesTitle && <h4 className={styles["perfisTitulo"]}>{profilesTitle}</h4>}

          <div className={styles["perfisLista"]} role="listbox" aria-label={profilesTitle ?? "Perfis"}>
            {profiles?.map((perfil) => {
              const inicial = perfil.initial ?? perfil.name.charAt(0).toUpperCase();

              const handleClick = () => {
                onSelectProfile?.(perfil);
              };

              const handleKeyDown = (e: KeyboardEvent<HTMLButtonElement>) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  handleClick();
                }
              };

              return (
                <button
                  key={perfil.id}
                  type="button"
                  role="option"
                  aria-selected={Boolean(perfil.active)}
                  className={cx(styles["perfilItem"], perfil.active && styles["perfilItemAtivo"])}
                  onClick={handleClick}
                  onKeyDown={handleKeyDown}
                >
                  <span className={styles["avatar"]} aria-hidden="true">
                    {inicial}
                  </span>
                  <span className={styles["perfilInfo"]}>
                    <strong className={styles["perfilNome"]}>{perfil.name}</strong>
                    {perfil.detail && <small className={styles["perfilDetalhe"]}>{perfil.detail}</small>}
                  </span>
                </button>
              );
            })}

            {addProfile && (
              <button
                type="button"
                className={cx(styles["perfilItem"], styles["botaoAdicionar"])}
                onClick={addProfile.onClick}
                aria-label={addProfile.label}
              >
                <span className={styles["avatarAdicionar"]} aria-hidden="true">
                  <Icon name="plus" size="md" />
                </span>
                <span className={styles["perfilInfo"]}>
                  <strong className={styles["perfilNome"]}>{addProfile.label}</strong>
                </span>
              </button>
            )}
          </div>
        </aside>
      )}

      {/* Coluna direita: Grupos de limites e acesso */}
      {temColunaControles && (
        <div className={styles["colunaControles"]}>
          {/* Seção Hoje com barra de progresso */}
          {today && (
            <section className={styles["hojeSecao"]} aria-label={today.title}>
              <h4 className={styles["hojeTitulo"]}>{today.title}</h4>
              <div className={styles["hojeCard"]}>
                <div className={styles["hojeTextos"]}>
                  <strong className={styles["hojeTempoUsado"]}>{today.timeUsed}</strong>
                  {today.timeRemaining && <span className={styles["hojeTempoRestante"]}>{today.timeRemaining}</span>}
                </div>
                <div
                  className={styles["progressoTrilho"]}
                  role="progressbar"
                  aria-valuenow={Math.round(progressoClamped * 100)}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-label={today.ariaLabel ?? `${today.timeUsed} (${Math.round(progressoClamped * 100)}%)`}
                >
                  <b
                    className={styles["progressoPreenchimento"]}
                    style={{ "--progresso": progressoClamped } as CSSProperties}
                  />
                </div>
              </div>
            </section>
          )}

          {/* Grupo Limites */}
          {limits && (
            <GrupoDeConfiguracao title={limits.title}>
              {limits.timePerDay && (
                <LinhaDeConfiguracao
                  title={limits.timePerDay.label}
                  {...(limits.timePerDay.description ? { description: limits.timePerDay.description } : {})}
                  control={
                    <CampoDeSelecao
                      label={limits.timePerDay.label}
                      options={limits.timePerDay.options}
                      value={limits.timePerDay.value}
                      onChange={limits.timePerDay.onChange}
                    />
                  }
                />
              )}
              {limits.schedule && (
                <LinhaDeConfiguracao
                  title={limits.schedule.label}
                  {...(limits.schedule.description ? { description: limits.schedule.description } : {})}
                  control={
                    <CampoDeSelecao
                      label={limits.schedule.label}
                      options={limits.schedule.options}
                      value={limits.schedule.value}
                      onChange={limits.schedule.onChange}
                    />
                  }
                />
              )}
              {limits.rating && (
                <LinhaDeConfiguracao
                  title={limits.rating.label}
                  {...(limits.rating.description ? { description: limits.rating.description } : {})}
                  control={
                    <CampoDeSelecao
                      label={limits.rating.label}
                      options={limits.rating.options}
                      value={limits.rating.value}
                      onChange={limits.rating.onChange}
                    />
                  }
                />
              )}
            </GrupoDeConfiguracao>
          )}

          {/* Grupo Acesso */}
          {access && (
            <GrupoDeConfiguracao title={access.title}>
              {access.pin && (
                <LinhaDeConfiguracao
                  title={access.pin.label}
                  {...(access.pin.description ? { description: access.pin.description } : {})}
                  control={({ labelId, descriptionId }) => (
                    <Chave
                      checked={access.pin!.checked}
                      onChange={access.pin!.onChange}
                      aria-labelledby={labelId}
                      aria-describedby={descriptionId}
                    />
                  )}
                />
              )}
              {access.gamesAboveRating && (
                <LinhaDeConfiguracao
                  title={access.gamesAboveRating.label}
                  {...(access.gamesAboveRating.description ? { description: access.gamesAboveRating.description } : {})}
                  control={
                    <CampoDeSelecao
                      label={access.gamesAboveRating.label}
                      options={access.gamesAboveRating.options}
                      value={access.gamesAboveRating.value}
                      onChange={access.gamesAboveRating.onChange}
                    />
                  }
                />
              )}
            </GrupoDeConfiguracao>
          )}
        </div>
      )}
    </div>
  );
}
