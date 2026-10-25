import { useCallback, useState } from "react";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { Marca } from "../Marca/Marca";
import { EntradaDoApp } from "./EntradaDoApp";
import styles from "./EntradaDoApp.demo.module.css";
import parentStyles from "./EntradaDoApp.module.css";

export const title = "EntradaDoApp";
export const preview = "EntradaDoApp";

const GAMES = [
  { id: "gow", initial: "G", name: "God of War: Ragnarök", active: true },
  { id: "elden", initial: "E", name: "Elden Ring", active: false },
  { id: "hades", initial: "H", name: "Hades II", active: false },
  { id: "cyber", initial: "C", name: "Cyberpunk 2077", active: false },
  { id: "sekiro", initial: "S", name: "Sekiro: Shadows Die Twice", active: false },
  { id: "witcher", initial: "T", name: "The Witcher 3: Wild Hunt", active: false },
];

export function Demo() {
  const [iteration, setIteration] = useState(0);
  const [doneCount, setDoneCount] = useState(0);
  const [active, setActive] = useState(true);

  const handleDone = useCallback(() => {
    setDoneCount((count) => count + 1);
    setActive(false);
  }, []);

  const handleRepeat = useCallback(() => {
    setActive(true);
    setIteration((i) => i + 1);
  }, []);

  return (
    <div className={styles["demoRoot"]}>
      <div className={styles["toolbar"]}>
        <BotaoDesktop variant="primary" onClick={handleRepeat}>
          Repetir
        </BotaoDesktop>
        <span className={styles["status"]}>
          Situação:{" "}
          <strong className={styles["statusHighlight"]}>
            {active ? "Reproduzindo abertura..." : "Concluído (onDone disparado)"}
          </strong>{" "}
          · Execuções concluídas: {doneCount}
        </span>
        <span className={styles["instructions"]}>
          (Clique na tela ou pressione Esc, Enter ou qualquer tecla para pular)
        </span>
      </div>

      <div className={styles["previewScroll"]}>
        <div className={styles["previewStage"]} data-theme="tako">
          <EntradaDoApp key={iteration} onDone={handleDone} texto="Iniciando TAKO...">
            <div className={styles["window"]}>
              <header
                className={`${styles["titlebar"]} ${parentStyles["staggerItem"]}`}
                style={{ "--step": 0 } as React.CSSProperties}
              >
                <div className={styles["titlebarLeft"]}>
                  <Marca variant="bar" />
                  <nav className={styles["tabs"]}>
                    <span className={styles["tab"]}>Biblioteca</span>
                  </nav>
                </div>
                <div className={styles["titlebarRight"]}>
                  <div className={styles["windowControls"]}>
                    <div className={styles["controlDot"]} />
                    <div className={styles["controlDot"]} />
                    <div className={styles["controlDot"]} />
                  </div>
                </div>
              </header>

              <div className={styles["body"]}>
                <aside
                  className={`${styles["sidebar"]} ${parentStyles["staggerItem"]}`}
                  style={{ "--step": 1 } as React.CSSProperties}
                >
                  <div className={styles["sidebarTitle"]}>Coleções · Todos</div>
                  <ul className={styles["gameList"]}>
                    {GAMES.map((game) => (
                      <li
                        key={game.id}
                        className={`${styles["gameRow"]} ${game.active ? styles["gameRowActive"] : ""}`}
                      >
                        <span className={styles["gameIcon"]}>{game.initial}</span>
                        <span>{game.name}</span>
                      </li>
                    ))}
                  </ul>
                </aside>

                <main className={styles["mainContent"]}>
                  <section
                    className={`${styles["hero"]} ${parentStyles["staggerItem"]}`}
                    style={{ "--step": 2 } as React.CSSProperties}
                  >
                    <span className={styles["heroTag"]}>Pronto para jogar</span>
                    <h1 className={styles["heroTitle"]}>God of War: Ragnarök</h1>
                    <div className={styles["heroStats"]}>
                      <div className={styles["statItem"]}>
                        <span className={styles["statValue"]}>80 h</span>
                        <span className={styles["statLabel"]}>Tempo de jogo</span>
                      </div>
                      <div className={styles["statItem"]}>
                        <span className={styles["statValue"]}>93%</span>
                        <span className={styles["statLabel"]}>Avaliação</span>
                      </div>
                      <div className={styles["statItem"]}>
                        <span className={styles["statValue"]}>51K</span>
                        <span className={styles["statLabel"]}>Jogadores ativos</span>
                      </div>
                    </div>
                  </section>

                  <section
                    className={`${styles["sectionShelf"]} ${parentStyles["staggerItem"]}`}
                    style={{ "--step": 3 } as React.CSSProperties}
                  >
                    <h2 className={styles["sectionHeading"]}>Jogados recentemente</h2>
                    <div className={styles["cardRow"]}>
                      {GAMES.slice(1, 5).map((game) => (
                        <div key={game.id} className={styles["card"]}>
                          <span>{game.name}</span>
                        </div>
                      ))}
                    </div>
                  </section>
                </main>
              </div>
            </div>
          </EntradaDoApp>
        </div>
      </div>
    </div>
  );
}
