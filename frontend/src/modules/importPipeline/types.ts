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
