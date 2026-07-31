export type RunScript = {
  id: string;
  label: string;
  fileName: string;
  filePath: string;
  content: string;
};

export type RunSettings = {
  defaultScriptId: string | null;
  scripts: RunScript[];
};

export type ScriptInput = {
  id: string | null;
  label: string;
  content: string;
  makeDefault: boolean;
};

export type RunState = {
  runId: string;
  projectId: string;
  scriptLabel: string;
  output: string;
  status: 'running' | 'stopping' | 'succeeded' | 'failed' | 'cancelled';
  exitCode: number | null;
};
