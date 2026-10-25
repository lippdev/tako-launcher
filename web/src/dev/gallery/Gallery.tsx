import type { ComponentType } from "react";
import { useAppearance, setAppearance, THEMES } from "../../appearance/appearance";
import styles from "./Gallery.module.css";

/**
 * Galeria de componentes (`#/componentes`) para validar cada peça contra a prévia do design system.
 * Cada componente traz a própria demo em `src/components/<Nome>/<Nome>.demo.tsx`, que exporta `title`, o nome da
 * prévia em `design/desktop/components/` (`preview`) e o componente `Demo`. Nada aqui precisa mudar por componente.
 */
interface DemoModule {
  title: string;
  preview: string;
  Demo: ComponentType;
}

const modules = import.meta.glob<DemoModule>("../../components/*/*.demo.tsx", { eager: true });
const demos = Object.values(modules).sort((a, b) => a.title.localeCompare(b.title, "pt-BR"));

export function Gallery() {
  const { theme } = useAppearance();
  return (
    <main className={styles.page}>
      <header className={styles.header}>
        <h1 className="page-title">Componentes</h1>
        <select
          aria-label="Tema"
          value={theme}
          onChange={(e) => {
            setAppearance({ theme: e.target.value as (typeof THEMES)[number] });
          }}
        >
          {THEMES.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
      </header>
      <nav className={styles.index}>
        {demos.map((d) => (
          <a key={d.title} href={`#/componentes`} onClick={() => document.getElementById(d.title)?.scrollIntoView()}>
            {d.title}
          </a>
        ))}
      </nav>
      {demos.map(({ title, preview, Demo }) => (
        <section key={title} id={title} className={styles.section}>
          <h2 className="title">{title}</h2>
          <p className="label-sm">Prévia: design/desktop/components/{preview}/preview.html</p>
          <div className={styles.stage}>
            <Demo />
          </div>
        </section>
      ))}
      {demos.length === 0 && <p className="body">Nenhum componente com demo ainda.</p>}
    </main>
  );
}
