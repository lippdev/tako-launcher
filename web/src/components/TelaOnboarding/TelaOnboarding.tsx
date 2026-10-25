import { type ReactNode } from "react";
import { cx } from "../../lib/cx";
import { Marca } from "../Marca/Marca";
import styles from "./TelaOnboarding.module.css";

export interface TelaOnboardingProps {
  skipLabel?: string;
  onSkip?: () => void;
  covers?: string[];
  children: ReactNode;
  className?: string;
}

const TOTAL_MURAL_COVERS = 24;

export function TelaOnboarding({
  skipLabel,
  onSkip,
  covers,
  children,
  className,
}: TelaOnboardingProps) {
  const muralItems = Array.from({ length: TOTAL_MURAL_COVERS }, (_, index) => {
    if (covers && covers.length > 0) {
      return covers[index % covers.length];
    }
    return undefined;
  });

  return (
    <div className={cx(styles["frame"], className)}>
      <div className={styles["mural"]} aria-hidden="true">
        {muralItems.map((coverSrc, i) => (
          <i
            key={i}
            className={styles["muralCard"]}
            style={coverSrc ? { backgroundImage: `url(${JSON.stringify(coverSrc)})` } : undefined}
          />
        ))}
      </div>
      <div className={styles["muralScrim"]} aria-hidden="true" />

      <div className={styles["brandContainer"]}>
        <Marca variant="bar" />
      </div>

      {skipLabel && (
        <button
          type="button"
          className={styles["skipButton"]}
          onClick={onSkip}
        >
          {skipLabel}
        </button>
      )}

      <main className={styles["contentColumn"]}>{children}</main>
    </div>
  );
}
