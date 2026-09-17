import { useEffect, useState } from "react";
import MainWindow from "./pages/MainWindow";
import Gallery from "./pages/Gallery";
import { reportWindowReady } from "./services/appService";
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

  // After the browser has actually painted, not merely after React committed:
  // a double rAF lands on the frame following the one this render produced,
  // which is the first moment there is something on screen to interact with.
  useEffect(() => {
    const frame = requestAnimationFrame(() =>
      requestAnimationFrame(reportWindowReady),
    );
    return () => cancelAnimationFrame(frame);
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
