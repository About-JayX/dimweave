import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface SelectOption {
  id: string;
  label: string;
}

export interface ClaudeConfig {
  installed: boolean;
  binaryPath: string;
  version: string;
  models: SelectOption[];
  effortLevels: SelectOption[];
  bridgePath: string;
}

const FALLBACK_CONFIG: ClaudeConfig = {
  installed: false,
  binaryPath: "",
  version: "",
  models: [],
  effortLevels: [],
  bridgePath: "",
};

export function useClaudeConfig() {
  const [config, setConfig] = useState<ClaudeConfig>(FALLBACK_CONFIG);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<ClaudeConfig>("detect_claude_config")
      .then(setConfig)
      .catch(() => setConfig(FALLBACK_CONFIG))
      .finally(() => setLoading(false));
  }, []);

  return { config, loading };
}
