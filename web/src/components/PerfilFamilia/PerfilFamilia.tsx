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
  nome: string;
  /** Descrição dos limites, ex.: "Responsável · sem limites" ou "Até 12 anos · 2 h por dia". */
  detalhe?: string | undefined;
  /** Inicial exibida no avatar (se omitida, usa a primeira letra do nome). */
  inicial?: string | undefined;
  /** Indica se é o perfil selecionado atualmente na tela. */
  ativo?: boolean | undefined;
}

export interface LimiteCampoConfig<T extends string = string> {
  label: string;
  descricao?: string | undefined;
  value: T;
  options: readonly OpcaoDeSelecao<T>[];
  onChange: (value: T) => void;
}

export interface ChaveCampoConfig {
  label: string;
  descricao?: string | undefined;
  checked: boolean;
  onChange: (checked: boolean) => void;
}

export interface BlocoHoje {
  /** Título da seção, ex.: "Hoje · Bia". */
  titulo: string;
  /** Tempo usado em destaque, ex.: "1 h 12 min de 2 h". */
  tempoUsado: string;
  /** Tempo restante ou horário limite, ex.: "Sobram 48 min até as 21:00". */
  tempoRestante?: string | undefined;
  /** Fração de 0 a 1 para preenchimento da barra de progresso. */
  progresso: number;
  /** Rótulo acessível da barra para leitores de tela. */
  ariaLabel?: string | undefined;
}

export interface BlocoLimites {
  /** Título do grupo, ex.: "Limites". */
  titulo: string;
  tempoPorDia?: LimiteCampoConfig | undefined;
  horario?: LimiteCampoConfig | undefined;
  classificacao?: LimiteCampoConfig | undefined;
}

export interface BlocoAcesso {
  /** Título do grupo, ex.: "Acesso". */
  titulo: string;
  pin?: ChaveCampoConfig | undefined;
  jogosAcima?: LimiteCampoConfig | undefined;
}

export interface PerfilFamiliaProps {
  /** Título do painel de perfis à esquerda, ex.: "Perfis neste PC". */
  perfisTitulo?: string | undefined;
  /** Lista de perfis do computador. */
  perfis?: readonly PerfilFamiliaItem[] | undefined;
  /** Callback ao selecionar um perfil. */
  onSelecionarPerfil?: ((perfil: PerfilFamiliaItem) => void) | undefined;
  /** Botão para adicionar novo perfil na lista à esquerda. */
  adicionarPerfil?:
    | {
        label: string;
        onClick: () => void;
      }
    | undefined;
  /** Bloco "Hoje" com barra de progresso do tempo usado no dia. */
  hoje?: BlocoHoje | undefined;
  /** Bloco "Limites" (tempo por dia, horário, classificação). */
  limites?: BlocoLimites | undefined;
  /** Bloco "Acesso" (PIN para sair do perfil, jogos acima da classificação). */
  acesso?: BlocoAcesso | undefined;
  className?: string | undefined;
}

/**
 * Componente PerfilFamilia (README do PerfilFamilia):
 * Exibe à esquerda os perfis do PC (com o ativo selecionado) e à direita os grupos de
 * configuração da família: Hoje (progresso), Limites e Acesso.
 * Sem dados, cada bloco correspondente desaparece.
 */
export function PerfilFamilia({
  perfisTitulo,
  perfis,
  onSelecionarPerfil,
  adicionarPerfil,
  hoje,
  limites,
  acesso,
  className,
}: PerfilFamiliaProps) {
  const temColunaPerfis = Boolean((perfis && perfis.length > 0) || adicionarPerfil);
  const temColunaControles = Boolean(hoje || limites || acesso);

  // Sem nenhum dado, o componente não renderiza
  if (!temColunaPerfis && !temColunaControles) {
    return null;
  }

  const progressoClamped = hoje ? Math.min(1, Math.max(0, hoje.progresso)) : 0;

  return (
    <div className={cx(styles["container"], className)}>
      {/* Coluna esquerda: Lista de perfis do computador */}
      {temColunaPerfis && (
        <aside className={styles["colunaPerfis"]} aria-label={perfisTitulo ?? "Perfis"}>
          {perfisTitulo && <h4 className={styles["perfisTitulo"]}>{perfisTitulo}</h4>}

          <div className={styles["perfisLista"]} role="listbox" aria-label={perfisTitulo ?? "Perfis"}>
            {perfis?.map((perfil) => {
              const inicial = perfil.inicial ?? perfil.nome.charAt(0).toUpperCase();

              const handleClick = () => {
                onSelecionarPerfil?.(perfil);
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
                  aria-selected={Boolean(perfil.ativo)}
                  className={cx(styles["perfilItem"], perfil.ativo && styles["perfilItemAtivo"])}
                  onClick={handleClick}
                  onKeyDown={handleKeyDown}
                >
                  <span className={styles["avatar"]} aria-hidden="true">
                    {inicial}
                  </span>
                  <span className={styles["perfilInfo"]}>
                    <strong className={styles["perfilNome"]}>{perfil.nome}</strong>
                    {perfil.detalhe && <small className={styles["perfilDetalhe"]}>{perfil.detalhe}</small>}
                  </span>
                </button>
              );
            })}

            {adicionarPerfil && (
              <button
                type="button"
                className={cx(styles["perfilItem"], styles["botaoAdicionar"])}
                onClick={adicionarPerfil.onClick}
                aria-label={adicionarPerfil.label}
              >
                <span className={styles["avatarAdicionar"]} aria-hidden="true">
                  <Icon name="plus" size="md" />
                </span>
                <span className={styles["perfilInfo"]}>
                  <strong className={styles["perfilNome"]}>{adicionarPerfil.label}</strong>
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
          {hoje && (
            <section className={styles["hojeSecao"]} aria-label={hoje.titulo}>
              <h4 className={styles["hojeTitulo"]}>{hoje.titulo}</h4>
              <div className={styles["hojeCard"]}>
                <div className={styles["hojeTextos"]}>
                  <strong className={styles["hojeTempoUsado"]}>{hoje.tempoUsado}</strong>
                  {hoje.tempoRestante && <span className={styles["hojeTempoRestante"]}>{hoje.tempoRestante}</span>}
                </div>
                <div
                  className={styles["progressoTrilho"]}
                  role="progressbar"
                  aria-valuenow={Math.round(progressoClamped * 100)}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-label={hoje.ariaLabel ?? `${hoje.tempoUsado} (${Math.round(progressoClamped * 100)}%)`}
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
          {limites && (
            <GrupoDeConfiguracao title={limites.titulo}>
              {limites.tempoPorDia && (
                <LinhaDeConfiguracao
                  title={limites.tempoPorDia.label}
                  {...(limites.tempoPorDia.descricao ? { description: limites.tempoPorDia.descricao } : {})}
                  control={
                    <CampoDeSelecao
                      label={limites.tempoPorDia.label}
                      options={limites.tempoPorDia.options}
                      value={limites.tempoPorDia.value}
                      onChange={limites.tempoPorDia.onChange}
                    />
                  }
                />
              )}
              {limites.horario && (
                <LinhaDeConfiguracao
                  title={limites.horario.label}
                  {...(limites.horario.descricao ? { description: limites.horario.descricao } : {})}
                  control={
                    <CampoDeSelecao
                      label={limites.horario.label}
                      options={limites.horario.options}
                      value={limites.horario.value}
                      onChange={limites.horario.onChange}
                    />
                  }
                />
              )}
              {limites.classificacao && (
                <LinhaDeConfiguracao
                  title={limites.classificacao.label}
                  {...(limites.classificacao.descricao ? { description: limites.classificacao.descricao } : {})}
                  control={
                    <CampoDeSelecao
                      label={limites.classificacao.label}
                      options={limites.classificacao.options}
                      value={limites.classificacao.value}
                      onChange={limites.classificacao.onChange}
                    />
                  }
                />
              )}
            </GrupoDeConfiguracao>
          )}

          {/* Grupo Acesso */}
          {acesso && (
            <GrupoDeConfiguracao title={acesso.titulo}>
              {acesso.pin && (
                <LinhaDeConfiguracao
                  title={acesso.pin.label}
                  {...(acesso.pin.descricao ? { description: acesso.pin.descricao } : {})}
                  control={({ labelId, descriptionId }) => (
                    <Chave
                      checked={acesso.pin!.checked}
                      onChange={acesso.pin!.onChange}
                      aria-labelledby={labelId}
                      aria-describedby={descriptionId}
                    />
                  )}
                />
              )}
              {acesso.jogosAcima && (
                <LinhaDeConfiguracao
                  title={acesso.jogosAcima.label}
                  {...(acesso.jogosAcima.descricao ? { description: acesso.jogosAcima.descricao } : {})}
                  control={
                    <CampoDeSelecao
                      label={acesso.jogosAcima.label}
                      options={acesso.jogosAcima.options}
                      value={acesso.jogosAcima.value}
                      onChange={acesso.jogosAcima.onChange}
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
