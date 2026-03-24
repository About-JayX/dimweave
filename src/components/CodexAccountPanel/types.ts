import type { CodexAccountInfo } from "@/types";

export interface DropdownOption {
  value: string;
  label: string;
  description?: string;
}

export interface CodexProfile {
  name?: string;
  planType?: string;
}

export interface UsageWindow {
  usedPercent: number;
  remainingPercent: number;
  windowMinutes: number | null;
}

export interface UsageSnapshot {
  source: string;
  allowed: boolean;
  limitReached: boolean;
  primary: UsageWindow | null;
  secondary: UsageWindow | null;
}

export interface CodexAccountPanelProps {
  profile: CodexProfile | null;
  usage: UsageSnapshot | null;
  refreshing: boolean;
  onRefresh: () => void;
  protocolData?: CodexAccountInfo;
  locked?: boolean;
}
