export type BackgroundJob = {
  id: number;
  jobType: string;
  status: string;
  progress: number;
  paramsJson: string | null;
  resultJson: string | null;
  errorMessage: string | null;
  createdAt: string;
  updatedAt: string;
};

export type BackupReport = {
  job: BackgroundJob;
  backupId: string;
  backupDir: string;
  manifestPath: string;
  databasePath: string;
  imagesPath: string | null;
  configPath: string | null;
};

export type RestoreReport = {
  job: BackgroundJob;
  restoredFrom: string;
  safetyBackupDir: string;
  databaseRestored: boolean;
  imagesRestored: boolean;
  configRestored: boolean;
  rebuildSearchIndexNote: string;
};

export type MaintenanceReport = {
  job: BackgroundJob;
  action: string;
  message: string;
  affectedRows: number | null;
  outputPath: string | null;
};
