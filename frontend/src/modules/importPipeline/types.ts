export type Mapping = Record<string, string>;

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

export type FieldMappingTemplate = {
  id: number;
  name: string;
  targetType: string;
  sourceHeadersJson: string;
  mappingJson: string;
  createdAt: string;
  updatedAt: string;
};
