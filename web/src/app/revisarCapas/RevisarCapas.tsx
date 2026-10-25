import { useEffect, useState } from "react";
import { call } from "../../bridge";
import type { ArtReviewItem } from "../../bridge/generated/ArtReviewItem";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { PilulaDaLoja } from "../../components/PilulaDaLoja/PilulaDaLoja";
import { DialogoSeletorDeImagens } from "../jogo/DialogoSeletorDeImagens";
import { useT } from "../../i18n";
import { libraryStrings } from "../strings";
import styles from "./RevisarCapas.module.css";

export interface RevisarCapasProps {
  onConcluir?: () => void;
  onVoltar?: () => void;
  className?: string;
}

export function RevisarCapas({
  onConcluir,
  onVoltar,
  className,
}: RevisarCapasProps) {
  const t = useT();
  const [loading, setLoading] = useState(true);
  const [total, setTotal] = useState(0);
  const [items, setItems] = useState<ArtReviewItem[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    call("art.review")
      .then((view) => {
        if (!cancelled) {
          setTotal(view.total);
          setItems(view.items);
          setLoading(false);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          console.error("erro ao carregar revisão de capas", err);
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const currentItem = items[currentIndex];

  const handleAvancarAposAcao = (gameId: string, mensagem: string) => {
    setNotice(mensagem);
    setItems((prev) => {
      const filtered = prev.filter((item) => item.gameId !== gameId);
      setCurrentIndex((idx) => {
        if (filtered.length === 0) return 0;
        return idx >= filtered.length ? filtered.length - 1 : idx;
      });
      return filtered;
    });
  };

  const handleAceitar = async (item: ArtReviewItem) => {
    try {
      await call("art.review.accept", { gameId: item.gameId });
      handleAvancarAposAcao(
        item.gameId,
        t("art_review.accepted", { game: item.title }) || `Capa de ${item.title} confirmada`,
      );
    } catch (err) {
      console.error("erro ao aceitar capa", err);
    }
  };

  const handleTrocar = () => {
    setPickerOpen(true);
  };

  const handleImagemTrocada = (item: ArtReviewItem) => {
    setPickerOpen(false);
    handleAvancarAposAcao(
      item.gameId,
      t("art_review.accepted", { game: item.title }) || `Capa de ${item.title} trocada`,
    );
  };

  const titulo = t("art_review.title") || "Revisar capas";
  const descricao =
    t("art_review.desc") ||
    "As fontes casam a biblioteca sozinhas; aqui ficam só os jogos que valem conferir";

  const renderBarrasConfianca = (confidence: ArtReviewItem["confidence"]) => {
    return (
      <div className={styles["barrasConfianca"]}>
        <div
          className={`${styles["barra"]} ${
            confidence === "high"
              ? styles["barraAtivaHigh"]
              : confidence === "medium"
                ? styles["barraAtivaMedium"]
                : styles["barraAtivaLow"]
          }`}
        />
        <div
          className={`${styles["barra"]} ${
            confidence === "high"
              ? styles["barraAtivaHigh"]
              : confidence === "medium"
                ? styles["barraAtivaMedium"]
                : ""
          }`}
        />
        <div
          className={`${styles["barra"]} ${
            confidence === "high" ? styles["barraAtivaHigh"] : ""
          }`}
        />
      </div>
    );
  };

  return (
    <div className={`${styles["revisarCapas"]} ${className ?? ""}`.trim()}>
      <header className={styles["cabecalho"]}>
        <div className={styles["cabecalhoTextos"]}>
          <h1 className={styles["titulo"]}>{titulo}</h1>
          {items.length > 0 && (
            <p className={styles["resumo"]}>
              {t("art_review.summary", { count: items.length, total }) ||
                `${items.length} de ${total} jogos para conferir`}
            </p>
          )}
          <p className={styles["descricao"]}>{descricao}</p>
        </div>
        <div className={styles["cabecalhoAcoes"]}>
          {onVoltar && (
            <BotaoDesktop variant="outlined" onClick={onVoltar}>
              Voltar
            </BotaoDesktop>
          )}
          {onConcluir && (
            <BotaoDesktop variant="primary" onClick={onConcluir}>
              Concluir
            </BotaoDesktop>
          )}
        </div>
      </header>

      {notice && (
        <div className={styles["notificacao"]} role="status">
          {notice}
        </div>
      )}

      {loading && (
        <div className={styles["carregando"]} role="status">
          <p>{t("art_review.computing") || "Conferindo a biblioteca…"}</p>
        </div>
      )}

      {!loading && items.length === 0 && (
        <div className={styles["vazio"]}>
          <h2 className={styles["vazioTitulo"]}>
            {t("art_review.all_good", { total }) ||
              `Os ${total} jogos estão com as imagens e as correspondências em ordem`}
          </h2>
          <p className={styles["descricao"]}>
            {t("art_review.text") ||
              "Capa, fundo, logo e ícone vêm sozinhos das fontes ligadas. Confira os jogos achados só pelo nome e os que ficaram sem alguma imagem."}
          </p>
          {onConcluir && (
            <BotaoDesktop variant="primary" onClick={onConcluir}>
              Concluir
            </BotaoDesktop>
          )}
        </div>
      )}

      {!loading && currentItem && (
        <article className={styles["cartaoRevisao"]}>
          <div className={styles["cartaoCabecalho"]}>
            <div className={styles["jogoInfo"]}>
              <h2 className={styles["jogoTitulo"]}>{currentItem.title}</h2>
              {currentItem.store && (
                <PilulaDaLoja label={libraryStrings.stores[currentItem.store] ?? currentItem.store} />
              )}
            </div>
            <span className={styles["contador"]}>
              {currentIndex + 1} de {items.length}
            </span>
          </div>

          <div className={styles["motivoAviso"]}>
            {currentItem.reason}
          </div>

          <div className={styles["comparacaoArtes"]}>
            <div className={styles["colunaArte"]}>
              <span className={styles["rotuloColuna"]}>Atual</span>
              <div className={styles["molduraCapa"]}>
                {currentItem.currentCover ? (
                  <img
                    src={currentItem.currentCover}
                    alt={`Capa atual de ${currentItem.title}`}
                    className={styles["capaImg"]}
                  />
                ) : (
                  <div className={styles["semCapa"]}>
                    <span>{t("art_review.no_cover") || "Sem capa"}</span>
                  </div>
                )}
              </div>
            </div>

            {currentItem.proposedCover && (
              <div className={styles["colunaArte"]}>
                <span className={styles["rotuloColuna"]}>Proposta</span>
                <div className={styles["molduraCapa"]}>
                  <img
                    src={currentItem.proposedCover}
                    alt={`Capa proposta de ${currentItem.title}`}
                    className={styles["capaImg"]}
                  />
                </div>
                <div className={styles["confiancaContainer"]}>
                  {renderBarrasConfianca(currentItem.confidence)}
                  <span className={styles["textoConfianca"]}>
                    {t(`art_review.conf_${currentItem.confidence}`, {
                      percent: currentItem.confidencePercent,
                    }) || `${currentItem.confidencePercent}% de confiança`}
                  </span>
                </div>
              </div>
            )}
          </div>

          <div className={styles["cartaoRodape"]}>
            <div className={styles["navegacao"]}>
              <BotaoDesktop
                variant="outlined"
                disabled={currentIndex <= 0}
                onClick={() => setCurrentIndex((idx) => Math.max(0, idx - 1))}
              >
                Anterior
              </BotaoDesktop>
              <BotaoDesktop
                variant="outlined"
                disabled={currentIndex >= items.length - 1}
                onClick={() => setCurrentIndex((idx) => Math.min(items.length - 1, idx + 1))}
              >
                Próximo
              </BotaoDesktop>
            </div>

            <div className={styles["acoesPrincipais"]}>
              <BotaoDesktop variant="outlined" onClick={handleTrocar}>
                {t("art_review.other") || "Escolher outra"}
              </BotaoDesktop>
              <BotaoDesktop
                variant="primary"
                onClick={() => {
                  void handleAceitar(currentItem);
                }}
              >
                {t("art_review.accept") || "Aceitar"}
              </BotaoDesktop>
            </div>
          </div>

          {pickerOpen && (
            <DialogoSeletorDeImagens
              gameId={currentItem.gameId}
              gameTitle={currentItem.title}
              open={pickerOpen}
              tipoInicial="capa"
              onClose={() => setPickerOpen(false)}
              onImageChanged={() => handleImagemTrocada(currentItem)}
            />
          )}
        </article>
      )}
    </div>
  );
}
