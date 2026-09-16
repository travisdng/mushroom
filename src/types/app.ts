/** Mirrors the `AppInfo` struct returned by the `ping` command (task 8). */
export type AppInfo = {
  name: string;
  version: string;
  platform: string;
  dataDir: string;
};

/**
 * Build-time version, used until `ping` exists. Vite inlines this from
 * package.json via `define` in vite.config.ts.
 */
export const APP_VERSION: string = __APP_VERSION__;
