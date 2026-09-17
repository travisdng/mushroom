import { DOCS_BASE } from "../types/app";

/**
 * Open a documentation page in the user's browser.
 *
 * The docs are not shipped inside the installer — they are the ones in the
 * repository, so a link always points at the current version rather than at
 * whatever was true when the user installed. The cost is that this needs the
 * network; the pages it points at are all things you read *before* you are
 * stuck, and the one thing you might need offline (the shortcut list) is a
 * dialog in the app rather than a link.
 */
export async function openDocs(page: string): Promise<void> {
  const url = `${DOCS_BASE}/${page}`;
  try {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } catch {
    // Running in a plain browser (npm run dev), or the plugin refused.
    window.open(url, "_blank", "noopener");
  }
}
