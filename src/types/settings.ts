export type GitHubStatus = {
  installed: boolean;
  authenticated: boolean;
  account: string | null;
  version: string | null;
  error: string | null;
};

export type AgentKind = 'amp' | 'codex' | 'custom';

export type AgentSettings = {
  preferredAgent: AgentKind | null;
  customName: string;
  customCommand: string;
};

export type AgentInfo = {
  kind: AgentKind;
  label: string;
  available: boolean;
  executable: string | null;
  version: string | null;
};

export type AgentConfiguration = {
  settings: AgentSettings;
  agents: AgentInfo[];
};
