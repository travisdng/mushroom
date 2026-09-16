import { useEffect, useState } from "react";
import MainWindow from "./pages/MainWindow";
import Gallery from "./pages/Gallery";
import { ShellProvider } from "./hooks/useShell";
import { NotesProvider } from "./hooks/useNotes";
import { SearchProvider } from "./hooks/useSearch";

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
      <NotesProvider>
        <SearchProvider>
          <MainWindow />
        </SearchProvider>
      </NotesProvider>
    </ShellProvider>
  );
}
