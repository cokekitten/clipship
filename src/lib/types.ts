export interface Config {
  version: 1;
  mode: "ssh" | "local";
  host: string;
  port: number;
  username: string;
  private_key_path: string;
  remote_dir: string;
  shortcut: string;
  shortcut_double_tap: boolean;
  auto_cleanup: boolean;
}

export interface Status {
  kind: "idle" | "ok" | "error";
  message: string;
  detail?: string;
}

export interface SaveConfigResponse {
  warnings: string[];
}
