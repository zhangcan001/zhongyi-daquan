# 线程 G：AI 接口

你负责 AI 表、AI 设置页、ai_commands、AI 服务接口、禁用态提示和 OpenAI-compatible 调用路径。AI 默认关闭，配置后才真实调用。

验收：
1. AI 设置页面存在。
2. AI 默认关闭。
3. 可以保存 provider_type、base_url、model_name。
4. 无 AI 配置时软件完整可用。
5. AI 禁用态返回明确提示。
