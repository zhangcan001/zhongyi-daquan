export type AiProviderSettings = {
  id?: number | null;
  providerType: string;
  providerName?: string | null;
  baseUrl?: string | null;
  modelName?: string | null;
  timeoutSeconds?: number | null;
  maxTokens?: number | null;
  temperature?: number | null;
  enabled: boolean;
  createdAt?: string | null;
  updatedAt?: string | null;
};

export type AiProviderSettingsResponse = {
  settings: AiProviderSettings;
  message: string;
};

export type SaveAiProviderSettingsRequest = {
  providerType: string;
  providerName?: string | null;
  baseUrl?: string | null;
  modelName?: string | null;
  timeoutSeconds?: number | null;
  maxTokens?: number | null;
  temperature?: number | null;
};

export type AiCommandResponse = {
  enabled: boolean;
  status: string;
  taskId?: number | null;
  message: string;
};

export type FormulaSource = {
  title?: string | null;
  note?: string | null;
};

export type FormulaCard = {
  formulaName: string;
  itemId?: number | null;
  relatedPattern?: string | null;
  composition?: string | null;
  originalDosage?: string | null;
  ratio?: string | null;
  usage?: string | null;
  decoctionMethod?: string | null;
  originalText?: string | null;
  indications?: string | null;
  explanation?: string | null;
  contraindications?: string | null;
  annotationSnippets: string[];
  sources: FormulaSource[];
  missingComposition: boolean;
};

export type FormulaAiAnswer = {
  enabled: boolean;
  status: string;
  message: string;
  systemPrompt: string;
  formulaCards: FormulaCard[];
  retrievalScope: string[];
};

export type FormulaAiRequest = {
  question: string;
  relatedItemId?: number | null;
  mode?: string | null;
};
