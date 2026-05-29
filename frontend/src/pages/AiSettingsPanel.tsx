import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  AiCommandResponse,
  AiProviderSettings,
  AiProviderSettingsResponse,
  SaveAiProviderSettingsRequest,
} from "../modules/ai/types";

const disabledMessage = "当前版本未启用 AI 调用";

const defaultSettings: SaveAiProviderSettingsRequest = {
  providerType: "disabled",
  providerName: "",
  baseUrl: "",
  modelName: "",
  timeoutSeconds: 30,
  maxTokens: 1024,
  temperature: 0.2,
};

export function AiSettingsPanel() {
  const [form, setForm] = useState<SaveAiProviderSettingsRequest>(defaultSettings);
  const [message, setMessage] = useState(disabledMessage);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<AiProviderSettingsResponse>("get_ai_provider_settings")
      .then((response) => {
        setForm(settingsToForm(response.settings));
        setMessage(response.message);
      })
      .catch((cause) => setError(String(cause)));
  }, []);

  function updateField<Key extends keyof SaveAiProviderSettingsRequest>(
    key: Key,
    value: SaveAiProviderSettingsRequest[Key],
  ) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  function saveSettings() {
    setIsSaving(true);
    setError(null);
    invoke<AiProviderSettingsResponse>("save_ai_provider_settings", { settings: form })
      .then((response) => {
        setForm(settingsToForm(response.settings));
        setMessage(response.message);
      })
      .catch((cause) => setError(String(cause)))
      .finally(() => setIsSaving(false));
  }

  function testConnection() {
    setError(null);
    invoke<AiCommandResponse>("test_ai_connection")
      .then((response) => setMessage(response.message))
      .catch((cause) => setError(String(cause)));
  }

  return (
    <section className="section-band ai-settings">
      <div className="section-heading">
        <div>
          <h2>AI 设置</h2>
          <p>v0.1 仅保留本地配置入口，默认关闭，不进行模型调用。</p>
        </div>
        <span className="status-pill muted">默认关闭</span>
      </div>

      <div className="ai-form-grid">
        <label>
          Provider Type
          <select value={form.providerType} onChange={(event) => updateField("providerType", event.target.value)}>
            <option value="disabled">disabled</option>
            <option value="ollama">ollama</option>
            <option value="openai_compatible">openai_compatible</option>
            <option value="deepseek">deepseek</option>
            <option value="custom_http">custom_http</option>
          </select>
        </label>

        <label>
          Provider Name
          <input
            value={form.providerName ?? ""}
            onChange={(event) => updateField("providerName", event.target.value)}
            placeholder="本地占位名称"
          />
        </label>

        <label>
          Base URL
          <input
            value={form.baseUrl ?? ""}
            onChange={(event) => updateField("baseUrl", event.target.value)}
            placeholder="仅保存，不连接"
          />
        </label>

        <label>
          Model Name
          <input
            value={form.modelName ?? ""}
            onChange={(event) => updateField("modelName", event.target.value)}
            placeholder="仅保存，不调用"
          />
        </label>

        <label>
          Timeout Seconds
          <input
            min={1}
            type="number"
            value={form.timeoutSeconds ?? 30}
            onChange={(event) => updateField("timeoutSeconds", Number(event.target.value))}
          />
        </label>

        <label>
          Max Tokens
          <input
            min={1}
            type="number"
            value={form.maxTokens ?? 1024}
            onChange={(event) => updateField("maxTokens", Number(event.target.value))}
          />
        </label>

        <label>
          Temperature
          <input
            max={2}
            min={0}
            step={0.1}
            type="number"
            value={form.temperature ?? 0.2}
            onChange={(event) => updateField("temperature", Number(event.target.value))}
          />
        </label>
      </div>

      <div className="ai-actions">
        <button type="button" onClick={saveSettings} disabled={isSaving}>
          {isSaving ? "保存中" : "保存本地配置"}
        </button>
        <button type="button" onClick={testConnection}>
          测试占位接口
        </button>
      </div>

      {error ? <p className="error-text">{error}</p> : null}
      <p className="ai-message">{message}</p>
    </section>
  );
}

function settingsToForm(settings: AiProviderSettings): SaveAiProviderSettingsRequest {
  return {
    providerType: settings.providerType,
    providerName: settings.providerName ?? "",
    baseUrl: settings.baseUrl ?? "",
    modelName: settings.modelName ?? "",
    timeoutSeconds: settings.timeoutSeconds ?? 30,
    maxTokens: settings.maxTokens ?? 1024,
    temperature: settings.temperature ?? 0.2,
  };
}
