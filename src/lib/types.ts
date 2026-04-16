export interface Config {
  version: 1;
  host: string;
  port: number;
  username: string;
  private_key_path: string;
  remote_dir: string;
  shortcut: string;
}

export interface Status {
  kind: "idle" | "ok" | "error";
  message: string;
  detail?: string;
}

export interface SaveConfigResponse {
  warnings: string[];
}
