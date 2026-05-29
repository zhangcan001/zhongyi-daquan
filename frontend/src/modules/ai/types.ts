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
