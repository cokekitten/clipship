import { invoke } from "@tauri-apps/api/core";
import type { Config, SaveConfigResponse } from "./types";

export const loadConfig = () => invoke<Config>("load_config");
export const defaultPrivateKey = () => invoke<string | null>("default_private_key");
export const saveConfig = (cfg: Config) => invoke<SaveConfigResponse>("save_config", { cfg });
export const testConnection = (cfg: Config) => invoke<void>("test_connection", { cfg });
export const triggerUploadNow = () => invoke<string>("trigger_upload_now");
export const copyLastUploaded = () => invoke<string | null>("copy_last_uploaded");
export const getAutostart = () => invoke<boolean>("get_autostart");
export const setAutostart = (enabled: boolean) => invoke<void>("set_autostart", { enabled });
