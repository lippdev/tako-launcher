import { useEffect, useState } from "react";
import { DevPage } from "./dev/DevPage";
import { Gallery } from "./dev/gallery/Gallery";

/** Rota pela âncora (`#/dev`, `#/componentes`); as telas de verdade entram depois, em cima disto. */
function useRoute(): string {
  const [route, setRoute] = useState(() => window.location.hash.slice(1));
  useEffect(() => {
    const onChange = () => {
      setRoute(window.location.hash.slice(1));
    };
    window.addEventListener("hashchange", onChange);
    return () => {
      window.removeEventListener("hashchange", onChange);
    };
  }, []);
  return route;
}

export function App() {
  const route = useRoute();
  if (route === "/dev") return <DevPage />;
  if (route === "/componentes") return <Gallery />;
  return null;
}
