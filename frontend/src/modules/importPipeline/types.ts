export type Mapping = Record<string, string>;

export type ImportDetectionResult = {
  detectedType: string;
  confidence: number;
  reason: string;
  sampleFields: string[];
  recordCount: number;
};

export type FieldMappingSuggestion = {
  sourceField: string;
  targetField?: string | null;
  confidence: number;
  decision: "auto" | "confirm" | "ignore" | string;
  reason: string;
};

export type ImportParsedPreview = {
  headers: string[];
  rows: Record<string, unknown>[];
  detection: ImportDetectionResult;
  mappingSuggestions: FieldMappingSuggestion[];
  directImportReady: boolean;
  warnings: string[];
};

export type ImportPackageFile = {
  path: string;
  importType: string;
  target?: string | null;
  primary: boolean;
  role?: string | null;
  autoStage: boolean;
  description?: string | null;
  skipReason?: string | null;
  required: boolean;
  exists: boolean;
  recordCount?: number | null;
};

export type ImportPackageDescriptor = {
  packageRoot: string;
  packageName?: string | null;
  importProfile?: string | null;
  manifestFound: boolean;
  manifestPath?: string | null;
  files: ImportPackageFile[];
  primaryFiles: string[];
  auxiliaryFiles: ImportPackageFile[];
  autoStageFiles: string[];
  skippedManifestFiles: ImportPackageFile[];
  detectedType: string;
  recordCount: number;
  directImportReady: boolean;
  warnings: string[];
  errors: string[];
};

export type ImportBatchSummary = {
  batch: {
    id: number;
    fileName: string;
    importType: string;
    targetType: string;
    status: string;
    totalCount: number;
    parsedCount: number;
    validCount: number;
    warningCount: number;
    errorCount: number;
    createdAt: string;
  };
  totalRows: number;
  importableRows: number;
  warningRows: number;
  errorRows: number;
};

export type StagingIssue = {
  severity: string;
  issueCode: string;
  fieldName?: string | null;
  message: string;
  suggestion?: string | null;
};

export type StagingRow = {
  id: number;
  rowIndex: number;
  raw: Record<string, unknown>;
  mapped: Record<string, unknown>;
  normalized: Record<string, unknown>;
  status: string;
  errorMessage?: string | null;
  warningMessage?: string | null;
  issues: StagingIssue[];
};

export type StagingPage = {
  summary: ImportBatchSummary;
  rows: StagingRow[];
  page: number;
  pageSize: number;
};

export type ImportQualityReport = {
  batchId: number;
  detectedType: string;
  totalRows: number;
  importableRows: number;
  warningRows: number;
  errorRows: number;
  fieldCoverage: Record<string, number>;
  emptyFieldCounts: Record<string, number>;
  duplicateFingerprintCount: number;
  searchTermsImportedCount: number;
  searchableKeywordsChecked: Record<string, boolean>;
  suggestions: string[];
};

export type FieldMappingTemplate = {
  id: number;
  name: string;
  targetType: string;
  sourceHeadersJson: string;
  mappingJson: string;
  createdAt: string;
  updatedAt: string;
};

export type ImportPlanAction = {
  rowIndex: number;
  actionType: string;
  itemType?: string | null;
  name?: string | null;
  existingItemId?: number | null;
  confidence: number;
  reason: string;
  draftJson: Record<string, unknown> | null;
};

export type ImportPlan = {
  planId: string;
  packagePath: string;
  packageName?: string | null;
  importIntent: string;
  duplicatePolicy: string;
  totalRecords: number;
  createCount: number;
  updateCount: number;
  attachAnnotationCount: number;
  skipDuplicateCount: number;
  needsReviewCount: number;
  rejectInvalidCount: number;
  typeCounts: Record<string, number>;
  categoryCounts: Record<string, number>;
  missingFieldCounts: Record<string, number>;
  duplicateCodes: string[];
  keywordChecks: Record<string, boolean>;
  warnings: string[];
  actions: ImportPlanAction[];
  aiMessage?: string | null;
};

export type ExecuteImportPlanResult = {
  planId: string;
  importRunId?: number | null;
  createdCount: number;
  mergedCount: number;
  attachedAnnotationCount: number;
  skippedCount: number;
  needsReviewCount: number;
  rejectedCount: number;
  searchIndexRebuilt: boolean;
  reportJson: Record<string, unknown>;
  canRollback: boolean;
  warnings: string[];
};

export type ImportRunSummary = {
  id: number;
  packageName?: string | null;
  importIntent: string;
  packagePath?: string | null;
  status: string;
  totalRecords: number;
  createCount: number;
  updateCount: number;
  attachAnnotationCount: number;
  skipDuplicateCount: number;
  failedCount: number;
  createdAt: string;
  completedAt?: string | null;
  rolledBackAt?: string | null;
};

export type ImportRunReport = {
  importRun: ImportRunSummary;
  summary: Record<string, unknown>;
  warnings: string[];
  errors: string[];
};

export type RollbackImportRunResult = {
  importRunId: number;
  rolledBackChanges: number;
  skippedChanges: number;
  warnings: string[];
  searchIndexRebuilt: boolean;
};
