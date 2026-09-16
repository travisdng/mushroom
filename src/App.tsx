import { useEffect, useState } from "react";
import MainWindow from "./pages/MainWindow";
import Gallery from "./pages/Gallery";
import { ShellProvider } from "./hooks/useShell";

/** `#gallery` renders the temporary control gallery from task 11. */
export default function App() {
  const [hash, setHash] = useState(() => window.location.hash);

  useEffect(() => {
    const onHashChange = () => setHash(window.location.hash);
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  if (hash === "#gallery") return <Gallery />;

  return (
    <ShellProvider>
      <MainWindow />
    </ShellProvider>
  );
}
