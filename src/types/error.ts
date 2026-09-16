/** Mirrors `AppErrorDto` in src-tauri/src/error.rs. */
export type AppErrorDto = {
  code: string;
  title: string;
  message: string;
  detail?: string;
  hint?: string;
};
