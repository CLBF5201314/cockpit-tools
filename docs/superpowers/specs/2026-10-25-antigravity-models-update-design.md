# Antigravity 最新模型与配额体系适配设计文档

- **创建日期**：2026-10-25
- **状态**：已批准 (Approved)
- **目标组件**：Cockpit Tools (Antigravity 账号管理、配额展示与自动切换)

---

## 1. 背景与目标

Google Antigravity 在近期版本更新中升级了其底层模型架构与配额分组体系：
1. **上线全新旗舰模型**：`Gemini 4 Argon (Medium)`（拥有独立的周配额限额池）。
2. **常规主力模型更迭**：`Gemini 3.8 Flash (High)`（原 Gemini 3.0 / 3.1 逐步退役，Gemini Pro 已彻底移除）。
3. **Claude & GPT 思考模型升级**：新增 `Claude Opus 5.5 (Medium)`、`Claude Sonnet 5.5 (Medium)` 以及 `GPT-OSS 120B (Medium)`。
4. **视觉规范**：用户要求展示名称去除所有括号及括号内部内容（如 `(Medium)`、`(High)`、`(Thinking)` 等），使 UI 更加整洁易读。
5. **动态与容错机制**：即使 Google 官方后续变更模型 ID 后缀或常量代号，系统仍具备三层动态容错机制（API 动态 displayName 优先、族群通配正则兜底、未定义 ID 首字母大写格式化兜底）。

---

## 2. 模型清单与命名映射（Model Catalog）

### 2.1 核心模型映射表

所有展示名称统一通过全局正则清洗，彻底移除 `(...)` 后缀。

| 规范 ID (`id`) | 清洗后显示名称 (`displayName`) | 原生代号 / 占位符 | 支持别名 (`aliases`) | 归属配额池 |
| :--- | :--- | :--- | :--- | :--- |
| `gemini-4-argon` | **Gemini 4 Argon** | 官方动态下发 | `gemini-4-argon-medium`, `gemini-4-argon-high`, `gemini-argon-wtu-weekly`, `argon:weekly` | Gemini 4 Argon Weekly Limit (独立) |
| `gemini-3.8-flash-high` | **Gemini 3.8 Flash** | `MODEL_PLACEHOLDER_M318` | `gemini-3.8-flash`, `gemini-3.8-flash-medium`, `gemini-3.8-flash-low` | Gemini Models (Weekly & 5-Hour) |
| `claude-opus-5-5-medium` | **Claude Opus 5.5** | 官方动态下发 | `claude-opus-5-5`, `claude-opus-5-5-high`, `claude-opus-5-5-thinking` | Claude and GPT models (Weekly & 5-Hour) |
| `claude-sonnet-5-5-medium` | **Claude Sonnet 5.5** | 官方动态下发 | `claude-sonnet-5-5`, `claude-sonnet-5-5-high`, `claude-sonnet-5-5-max`, `claude-sonnet-5-5-low` | Claude and GPT models (Weekly & 5-Hour) |
| `gpt-oss-120b-medium` | **GPT-OSS 120B** | `MODEL_OPENAI_GPT_OSS_120B_MEDIUM` | `gpt-oss-120b` | Claude and GPT models (Weekly & 5-Hour) |

### 2.2 名称清洗规范

在 [`src/utils/modelNames.ts`](file:///e:/AI-based-project/cockpit/src/utils/modelNames.ts) 及 [`src/utils/antigravityModels.ts`](file:///e:/AI-based-project/cockpit/src/utils/antigravityModels.ts) 中增加清洗函数：

```typescript
export function sanitizeModelDisplayName(rawName: string): string {
  return rawName.replace(/\s*\([^)]*\)/g, '').trim();
}
```

---

## 3. 分组策略与自动切号（Auto-Switch Groups）

彻底废弃原 `Gemini Pro` 分组，将默认分组精简并重构为以下 3 个核心分组：

| 分组 ID (`id`) | 分组名称 (`name`) | 匹配规则（按正则与前缀匹配） | 说明 |
| :--- | :--- | :--- | :--- |
| **`g4_argon`** | **Gemini 4 Argon** | `/(?:^|[-_])argon(?:[-_]|$)/i` 或名称含 `argon` | 独立分组，对应独立的 `Gemini 4 Argon Weekly Limit` 额度池 |
| **`g3_flash`** | **Gemini Flash** | `/(?:^|[-_])flash(?:[-_]|$)/i` 或名称含 `flash` | 包含 `gemini-3.8-flash*`、`gemini-3-flash*` |
| **`claude_45`** | **Claude** | 匹配前缀 `claude-`、`gpt-` 或名称含 `claude` / `gpt` | 包含 Claude 5.5 系列、Claude 4.x 系列及 GPT-OSS |

---

## 4. 配额数据解析与卡片展示适配

### 4.1 Rust 核心配额收集
文件：[`crates/cockpit-core/src/modules/quota.rs`](file:///e:/AI-based-project/cockpit/crates/cockpit-core/src/modules/quota.rs)
- 在 `build_quota_data_from_response` 中，放宽模型配额筛选条件：
  ```rust
  if name.contains("gemini") || name.contains("claude") || name.contains("argon") || name.contains("gpt") {
      quota_data.add_model(name, display_name, percentage, reset_time);
  }
  ```

### 4.2 前端卡片展示呈现
文件：[`src/presentation/platformAccountPresentation.ts`](file:///e:/AI-based-project/cockpit/src/presentation/platformAccountPresentation.ts)
- `argonWeekly`: 精确匹配 `argon:weekly`、`gemini-argon-wtu-weekly` 或名称包含 `argon`。
- 展示标签呈现为：
  - `Claude (5h)` 与 `Claude (Weekly)`
  - `Gemini (5h)` 与 `Gemini (Weekly)`
  - `Gemini 4 Argon (Weekly)`
- 针对 Free Tier 账号自动隐藏 5h 配额条，仅显示 Weekly 限额条。

---

## 5. 本地代理配置（CLIProxyAPI）

文件：[`sidecars/cockpit-cliproxy/third_party/CLIProxyAPI/internal/registry/models/models.json`](file:///e:/AI-based-project/cockpit/sidecars/cockpit-cliproxy/third_party/CLIProxyAPI/internal/registry/models/models.json)
- 在 `antigravity` 模型列表中补充 `gemini-4-argon`、`claude-opus-5-5-medium`、`claude-sonnet-5-5-medium` 等模型元数据（`context_length`、`thinking` 等）。

---

## 6. 测试与验证策略

1. **前端单测**：
   - 运行并更新 [`src/presentation/antigravityQuotaPresentation.test.ts`](file:///e:/AI-based-project/cockpit/src/presentation/antigravityQuotaPresentation.test.ts)。
   - 验证名称去括号（无 `(...)`）、`argon` 独立限额展示与无 Pro 分组。
   - 执行 `npm run typecheck` 验证无类型错误。
2. **Rust 核心单测**：
   - 执行 `cargo test --package cockpit-core` 验证模型过滤与配额结构。
