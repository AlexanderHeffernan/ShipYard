export type ApplicationKind = 'editor' | 'terminal';

export type OpenApplication = {
  id: string;
  label: string;
  kind: ApplicationKind;
  appPath: string;
  available: boolean;
};

export type OpenApplicationInput = {
  id: string | null;
  label: string;
  kind: ApplicationKind;
  appPath: string;
  makeDefault: boolean;
};

export type OpenSettings = {
  defaultApplicationId: string | null;
  applications: OpenApplication[];
};
