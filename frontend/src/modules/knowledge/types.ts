export type KnowledgeType =
  | "herb"
  | "formula"
  | "acupuncture"
  | "syndrome"
  | "theory"
  | "note"
  | "meridian"
  | "acupoint"
  | "disease";
export type DataStatus = "draft" | "pending_review" | "reviewed" | "needs_check" | "imported" | "needs_fix" | "validated" | "ready" | "archived";
export type CompletenessStatus = "empty" | "partial" | "complete";

export type KnowledgeItem = {
  id?: number;
  itemType: KnowledgeType;
  code?: string | null;
  name: string;
  alias?: string | null;
  pinyin?: string | null;
  category?: string | null;
  summary?: string | null;
  content?: string | null;
  sourceNote?: string | null;
  tags?: string | null;
  dataStatus: DataStatus;
  completenessStatus: CompletenessStatus;
  contentVersion: number;
  isFavorite: boolean;
  detail?: Record<string, unknown> | null;
  importBatchId?: string | null;
  sourcePackage?: string | null;
  createdAt?: string | null;
  updatedAt?: string | null;
};

export type KnowledgeInput = Omit<
  KnowledgeItem,
  "id" | "contentVersion" | "createdAt" | "updatedAt"
> & {
  detail: Record<string, string | number | null>;
};

export type KnowledgeDetailResponse = {
  item: KnowledgeItem;
  detail: Record<string, string | number | null>;
  annotations: KnowledgeAnnotation[];
  notes: UserNote[];
  versions: Array<{
    id: number;
    itemId: number;
    versionNo: number;
    snapshotJson: string;
    changeSummary?: string | null;
    changedAt: string;
  }>;
};

export type KnowledgeAnnotation = {
  id: number;
  knowledgeItemId: number;
  annotationType: string;
  sourceTitle?: string | null;
  sourceNote?: string | null;
  content: string;
  detail?: Record<string, unknown> | null;
  tags?: string | null;
  createdAt: string;
  updatedAt: string;
};

export type RecentView = {
  id: number;
  itemId: number;
  itemName: string;
  itemType: KnowledgeType;
  category?: string | null;
  viewedAt: string;
};

export type FavoriteItem = {
  id: number;
  itemId: number;
  itemName: string;
  itemType: KnowledgeType;
  category?: string | null;
  createdAt: string;
};

export type UserNote = {
  id: number;
  itemId: number;
  noteText: string;
  createdAt: string;
  updatedAt: string;
};

export type DashboardStats = {
  knowledgeCount: number;
  annotationCount: number;
  importRunCount: number;
  favoriteCount: number;
  recentViewCount: number;
};

export type EnhancedSearchResult = {
  itemId: number;
  itemType: KnowledgeType;
  typeLabel: string;
  groupName: string;
  code?: string | null;
  name: string;
  category?: string | null;
  summary?: string | null;
  contentSnippet?: string | null;
  sourceTitle?: string | null;
  sourceNote?: string | null;
  tags?: string | null;
  hasAnnotations: boolean;
  annotationCount: number;
  annotationSnippet?: string | null;
  matchedBy: string;
  score: number;
  importBatchId?: string | null;
  sourcePackage?: string | null;
};

export type EnhancedSearchResponse = {
  query: string;
  filter: string;
  total: number;
  page: number;
  pageSize: number;
  durationMs: number;
  groups: Array<{
    groupName: string;
    results: EnhancedSearchResult[];
  }>;
};

export type KnowledgeListResponse = {
  total: number;
  page: number;
  pageSize: number;
  items: KnowledgeItem[];
};

export type GridSaveResponse = {
  savedCount: number;
  itemIds: number[];
  errors: Array<{
    rowIndex: number;
    fieldName: string;
    message: string;
  }>;
};

export const knowledgeTypeOptions: Array<{ value: KnowledgeType; label: string }> = [
  { value: "herb", label: "中药" },
  { value: "formula", label: "方剂" },
  { value: "acupoint", label: "穴位" },
  { value: "meridian", label: "经络" },
  { value: "acupuncture", label: "针灸" },
  { value: "syndrome", label: "辨证" },
  { value: "disease", label: "病症" },
  { value: "theory", label: "理论" },
  { value: "note", label: "笔记" },
];

export const dataStatusOptions: Array<{ value: DataStatus; label: string }> = [
  { value: "draft", label: "草稿" },
  { value: "pending_review", label: "待复核" },
  { value: "reviewed", label: "已复核" },
  { value: "needs_check", label: "需检查" },
  { value: "imported", label: "已导入" },
  { value: "needs_fix", label: "需修正" },
  { value: "validated", label: "已校验" },
  { value: "ready", label: "可使用" },
  { value: "archived", label: "已归档" },
];

export const completenessOptions: Array<{ value: CompletenessStatus; label: string }> = [
  { value: "empty", label: "空" },
  { value: "partial", label: "部分" },
  { value: "complete", label: "完整" },
];
