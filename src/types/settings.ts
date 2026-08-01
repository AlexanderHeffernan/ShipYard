export type GitHubStatus = {
  installed: boolean;
  authenticated: boolean;
  account: string | null;
  version: string | null;
  error: string | null;
};

export type AgentKind = 'amp' | 'codex';

export type AgentSettings = {
  preferredAgent: AgentKind | null;
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
