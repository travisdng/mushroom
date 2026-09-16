import { useEffect, useState } from "react";
import Gallery from "./pages/Gallery";

/**
 * Placeholder shell. The real window chrome — menu bar, toolbar, panels,
 * status bar — arrives in tasks 12-17.
 *
 * `#gallery` renders the temporary control gallery from task 11.
 */
export default function App() {
  const [hash, setHash] = useState(() => window.location.hash);

  useEffect(() => {
    const onHashChange = () => setHash(window.location.hash);
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  if (hash === "#gallery") return <Gallery />;

  return <div style={{ padding: 12 }}>Mushroom</div>;
}
