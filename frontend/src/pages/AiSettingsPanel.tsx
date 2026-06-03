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
  apiKey: "",
  modelName: "",
  timeoutSeconds: 30,
  maxTokens: 1200,
  temperature: 0.2,
  maxContextItems: 6,
  maxContextChars: 6000,
  onlyUseLocalContext: true,
  safetyMode: "strict",
  enabled: false,
};

export function AiSettingsPanel() {
  const [form, setForm] = useState<SaveAiProviderSettingsRequest>(defaultSettings);
  const [message, setMessage] = useState(disabledMessage);
  const [hasApiKey, setHasApiKey] = useState(false);
  const [apiKeyStatus, setApiKeyStatus] = useState("未配置");
  const [showApiKey, setShowApiKey] = useState(false);
  const [assistantQuestion, setAssistantQuestion] = useState("");
  const [assistantAnswer, setAssistantAnswer] = useState<AiCommandResponse | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [isAsking, setIsAsking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<AiProviderSettingsResponse>("get_ai_settings")
      .then((response) => {
        setForm(settingsToForm(response.settings));
        setHasApiKey(response.settings.hasApiKey);
        setApiKeyStatus(response.settings.hasApiKey ? "已配置" : "未配置");
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
    invoke<AiProviderSettingsResponse>("save_ai_settings", { settings: form })
      .then((response) => {
        setForm(settingsToForm(response.settings));
        setHasApiKey(response.settings.hasApiKey);
        setApiKeyStatus(form.apiKey?.trim() ? "已更新" : response.settings.hasApiKey ? "已配置" : "未配置");
        setMessage(response.message);
      })
      .catch((cause) => setError(String(cause)))
      .finally(() => setIsSaving(false));
  }

  function testConnection() {
    setIsTesting(true);
    setError(null);
    invoke<AiCommandResponse>("test_ai_connection")
      .then((response) => setMessage(response.message))
      .catch((cause) => setError(String(cause)))
      .finally(() => setIsTesting(false));
  }

  function clearApiKey() {
    setError(null);
    invoke<AiProviderSettingsResponse>("clear_ai_api_key")
      .then((response) => {
        setForm(settingsToForm(response.settings));
        setHasApiKey(false);
        setApiKeyStatus("已清除");
        setMessage(response.message);
      })
      .catch((cause) => setError(String(cause)));
  }

  function askAssistant() {
    const question = assistantQuestion.trim();
    if (!question) return;
    setIsAsking(true);
    setError(null);
    setAssistantAnswer(null);
    invoke<AiCommandResponse>("run_ai_task", {
      request: {
        taskType: "local_qa",
        inputJson: JSON.stringify({ question }),
        relatedBatchId: null,
        relatedRowId: null,
        relatedItemId: null,
      },
    })
      .then((response) => {
        setAssistantAnswer(response);
        setMessage(response.message);
      })
      .catch((cause) => setError(String(cause)))
      .finally(() => setIsAsking(false));
  }

  return (
    <section className="section-band ai-settings">
      <div className="section-heading">
        <div>
          <h2>AI 设置</h2>
          <p>AI 默认关闭。开启后，仅在你主动点击 AI 功能时发送当前问题和本地检索片段。</p>
        </div>
        <span className={form.enabled ? "status-pill" : "status-pill muted"}>
          {form.enabled ? "已启用" : "默认关闭"}
        </span>
      </div>
      <div className="ai-form-grid">
        <label>
          启用 AI
          <select
            value={form.enabled ? "true" : "false"}
            onChange={(event) => updateField("enabled", event.target.value === "true")}
          >
            <option value="false">关闭</option>
            <option value="true">开启</option>
          </select>
        </label>

        <label>
          Provider Type
          <select value={form.providerType} onChange={(event) => updateField("providerType", event.target.value)}>
            <option value="disabled">disabled</option>
            <option value="openai_compatible">openai_compatible</option>
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
          API Key
          <span className="api-key-row">
            <input
              type={showApiKey ? "text" : "password"}
              value={form.apiKey ?? ""}
              onChange={(event) => updateField("apiKey", event.target.value)}
              placeholder={hasApiKey ? "已配置，留空则不覆盖" : "请输入 API Key"}
            />
            <button type="button" onClick={() => setShowApiKey((value) => !value)}>
              {showApiKey ? "隐藏" : "显示"}
            </button>
          </span>
          <small>API Key 状态：{apiKeyStatus}</small>
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

        <label>
          Max Context Items
          <input
            min={1}
            max={20}
            type="number"
            value={form.maxContextItems ?? 6}
            onChange={(event) => updateField("maxContextItems", Number(event.target.value))}
          />
        </label>

        <label>
          Max Context Chars
          <input
            min={500}
            max={30000}
            type="number"
            value={form.maxContextChars ?? 6000}
            onChange={(event) => updateField("maxContextChars", Number(event.target.value))}
          />
        </label>
      </div>

      <div className="ai-actions">
        <button type="button" onClick={saveSettings} disabled={isSaving}>
          {isSaving ? "保存中" : "保存配置"}
        </button>
        <button type="button" onClick={testConnection} disabled={isTesting}>
          {isTesting ? "测试中" : "测试连接"}
        </button>
        <button type="button" onClick={clearApiKey} disabled={!hasApiKey}>
          清除 API Key
        </button>
      </div>

      {error ? <p className="error-text">{error}</p> : null}
      <p className="ai-message">{message}</p>

      <section className="ai-assistant-box">
        <h3>AI 知识库助手</h3>
        <div className="inline-action-row">
          <input
            value={assistantQuestion}
            onChange={(event) => setAssistantQuestion(event.target.value)}
            placeholder="输入资料问题，例如：桂枝汤组成是什么？"
          />
          <button type="button" onClick={askAssistant} disabled={isAsking || !assistantQuestion.trim()}>
            {isAsking ? "生成中" : "提问"}
          </button>
        </div>
        {assistantAnswer?.answer ? <div className="reader-content">{assistantAnswer.answer}</div> : null}
        {assistantAnswer?.citations?.length ? (
          <div className="formula-field">
            <strong>引用来源</strong>
            <ul>
              {assistantAnswer.citations.map((citation, index) => (
                <li key={`${citation.title ?? ""}-${citation.note ?? ""}-${index}`}>
                  {[citation.title, citation.note].filter(Boolean).join("｜") || "未记录来源"}
                </li>
              ))}
            </ul>
          </div>
        ) : null}
      </section>
    </section>
  );
}

function settingsToForm(settings: AiProviderSettings): SaveAiProviderSettingsRequest {
  return {
    providerType: settings.providerType,
    providerName: settings.providerName ?? "",
    baseUrl: settings.baseUrl ?? "",
    apiKey: "",
    modelName: settings.modelName ?? "",
    timeoutSeconds: settings.timeoutSeconds ?? 30,
    maxTokens: settings.maxTokens ?? 1200,
    temperature: settings.temperature ?? 0.2,
    maxContextItems: settings.maxContextItems ?? 6,
    maxContextChars: settings.maxContextChars ?? 6000,
    onlyUseLocalContext: settings.onlyUseLocalContext,
    safetyMode: settings.safetyMode ?? "strict",
    enabled: settings.enabled,
  };
}
