# Antigravity 最新模型与配额体系适配实施计划 (Implementation Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 适配 Antigravity 最新模型（Gemini 4 Argon、Gemini 3.8 Flash、Claude Opus 5.5、Claude Sonnet 5.5、GPT-OSS 120B），去除显示名称括号内容，移除 Gemini Pro 分组，新增独立 Gemini 4 Argon 分组，打通配额解析与本地代理字典。

**Architecture:** 前端更新模型规范定义与全局去括号清洗逻辑；重构默认分组定义（Argon 独立组、Flash 组、Claude 组，移除 Pro 组）；Rust 核心层同步放宽配额数据过滤器并对齐切号分组；Sidecar 本地代理词典追加新模型元数据；更新 TypeScript 与 Rust 单元测试。

**Tech Stack:** TypeScript, React 19, Tauri 2, Rust (cockpit-core), Go / JSON (CLIProxyAPI)

---

### Task 1: 前端依赖与现有单测基线确认

**Files:**
- Modify: `package.json` (只读确认)
- Command: `npm install`

- [ ] **Step 1: 安装前端依赖以激活测试环境**

```bash
npm install
```

- [ ] **Step 2: 验证单测工具可用性**

运行单测命令验证测试环境正常：
```bash
node --import tsx src/presentation/antigravityQuotaPresentation.test.ts
```
Expected: PASS 或明确的测试输出。

---

### Task 2: 前端模型定义与名称清洗逻辑（`modelNames.ts` & `antigravityModels.ts`）

**Files:**
- Modify: `src/utils/modelNames.ts`
- Modify: `src/utils/antigravityModels.ts`
- Test: `src/presentation/antigravityQuotaPresentation.test.ts`

- [ ] **Step 1: 在 `antigravityQuotaPresentation.test.ts` 编写名称清洗与新模型识别测试**

在 `src/presentation/antigravityQuotaPresentation.test.ts` 末尾添加：
```typescript
test('model display names strip parentheses and match new models correctly', () => {
  const { getAntigravityModelDisplayName } = require('../utils/antigravityModels');
  assert.equal(getAntigravityModelDisplayName('gemini-4-argon'), 'Gemini 4 Argon');
  assert.equal(getAntigravityModelDisplayName('gemini-3.8-flash-high'), 'Gemini 3.8 Flash');
  assert.equal(getAntigravityModelDisplayName('claude-opus-5-5-medium'), 'Claude Opus 5.5');
  assert.equal(getAntigravityModelDisplayName('claude-sonnet-5-5-medium'), 'Claude Sonnet 5.5');
  assert.equal(getAntigravityModelDisplayName('gpt-oss-120b-medium'), 'GPT-OSS 120B');
});
```

- [ ] **Step 2: 运行测试验证失败**

```bash
node --import tsx src/presentation/antigravityQuotaPresentation.test.ts
```
Expected: FAIL（未识别 `gemini-4-argon` 或带有括号）

- [ ] **Step 3: 更新 `src/utils/antigravityModels.ts`**

在 `CANONICAL_MODELS` 中移除旧的 `gemini-3.1-pro-*`，加入新模型，并在展示名上彻底去除括号：
```typescript
const CANONICAL_MODELS: CanonicalModel[] = [
  {
    id: 'gemini-4-argon',
    displayName: 'Gemini 4 Argon',
    modelConstant: null,
    aliases: ['gemini-4-argon-medium', 'gemini-4-argon-high', 'gemini-argon-wtu-weekly', 'argon:weekly'],
  },
  {
    id: 'gemini-3.8-flash-high',
    displayName: 'Gemini 3.8 Flash',
    modelConstant: 'MODEL_PLACEHOLDER_M318',
    aliases: ['gemini-3.8-flash', 'gemini-3.8-flash-medium', 'gemini-3.8-flash-low', 'gemini-3-flash'],
  },
  {
    id: 'claude-sonnet-5-5-medium',
    displayName: 'Claude Sonnet 5.5',
    modelConstant: null,
    aliases: ['claude-sonnet-5-5', 'claude-sonnet-5-5-high', 'claude-sonnet-5-5-max', 'claude-sonnet-5-5-low', 'claude-sonnet-4-6'],
  },
  {
    id: 'claude-opus-5-5-medium',
    displayName: 'Claude Opus 5.5',
    modelConstant: null,
    aliases: ['claude-opus-5-5', 'claude-opus-5-5-high', 'claude-opus-5-5-thinking', 'claude-opus-4-6-thinking'],
  },
  {
    id: 'gpt-oss-120b-medium',
    displayName: 'GPT-OSS 120B',
    modelConstant: 'MODEL_OPENAI_GPT_OSS_120B_MEDIUM',
    aliases: ['gpt-oss-120b'],
  },
];
```

在 `getAntigravityModelDisplayName` 中增加全局清洗：
```typescript
export const getAntigravityModelDisplayName = (value: string): string => {
  const found = resolveCanonicalModel({ id: value, modelConstant: value, displayName: value });
  if (found) return found.displayName;
  return value.replace(/\s*\([^)]*\)/g, '').trim();
};
```

- [ ] **Step 4: 更新 `src/utils/modelNames.ts` 显示名表与去括号格式化**

在 `MODEL_DISPLAY_NAMES` 中添加新模型，并在 `getModelDisplayName` 中加入去括号清洗：
```typescript
export function getModelDisplayName(modelId: string): string {
  const name = MODEL_DISPLAY_NAMES[modelId] || modelId
    .split('-')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
  return name.replace(/\s*\([^)]*\)/g, '').trim();
}
```

- [ ] **Step 5: 重新运行单测验证通过**

```bash
node --import tsx src/presentation/antigravityQuotaPresentation.test.ts
```
Expected: PASS

- [ ] **Step 6: 提交改动**

```bash
git add src/utils/antigravityModels.ts src/utils/modelNames.ts src/presentation/antigravityQuotaPresentation.test.ts
git commit -m "feat(antigravity): update canonical models and strip parentheses from display names"
```

---

### Task 3: 重构默认分组（移除 Gemini Pro，新增独立 Gemini 4 Argon 分组）

**Files:**
- Modify: `src/utils/modelNames.ts`
- Modify: `src/services/groupService.ts`
- Modify: `src-tauri/src/modules/group_settings.rs`
- Modify: `crates/cockpit-core/src/modules/account.rs`

- [ ] **Step 1: 更新 `src/utils/modelNames.ts` 的 `getDefaultGroups` 与正则匹配器**

将 `getDefaultGroups` 调整为 `claude_45`、`g3_flash`、`g4_argon`：
```typescript
const GEMINI_ARGON_ID_PATTERN = /(?:^|[-_])argon(?:[-_]|$)/i;
const GEMINI_ANY_FLASH_ID_PATTERN = /^gemini-\d+(?:\.\d+)?-flash(?:-|$)/i;

const DEFAULT_GROUP_PREFIX_MATCHERS: Record<string, GroupPrefixMatcher> = {
  g4_argon: (normalizedModelId, normalizedModelName) =>
    GEMINI_ARGON_ID_PATTERN.test(normalizedModelId) ||
    normalizedModelName.toLowerCase().includes('argon'),
  claude_45: (normalizedModelId, normalizedModelName) =>
    normalizedModelId.startsWith('claude-') ||
    normalizedModelId.startsWith('model_claude') ||
    normalizedModelId.startsWith('gpt-oss') ||
    normalizedModelName.startsWith('claude ') ||
    normalizedModelName.startsWith('gpt-oss'),
  g3_flash: (normalizedModelId, normalizedModelName) =>
    GEMINI_ANY_FLASH_ID_PATTERN.test(normalizedModelId) ||
    normalizedModelName.toLowerCase().includes('flash'),
};

export function getDefaultGroups(): DefaultGroup[] {
  return [
    {
      id: 'g4_argon',
      name: 'Gemini 4 Argon',
      desktopModels: ['gemini-4-argon', 'gemini-4-argon-medium', 'gemini-4-argon-high'],
      pluginModels: [],
    },
    {
      id: 'g3_flash',
      name: 'Gemini Flash',
      desktopModels: ['gemini-3.8-flash-high', 'gemini-3.8-flash-medium', 'gemini-3.8-flash-low', 'gemini-3.8-flash', 'gemini-3-flash'],
      pluginModels: ['MODEL_PLACEHOLDER_M318', 'MODEL_PLACEHOLDER_M18'],
    },
    {
      id: 'claude_45',
      name: 'Claude',
      desktopModels: [
        'claude-opus-5-5-medium',
        'claude-opus-5-5-high',
        'claude-sonnet-5-5-medium',
        'claude-sonnet-5-5-high',
        'claude-sonnet-4-6',
        'claude-opus-4-6-thinking',
        'gpt-oss-120b-medium',
      ],
      pluginModels: [
        'MODEL_PLACEHOLDER_M26',
        'MODEL_PLACEHOLDER_M35',
        'MODEL_OPENAI_GPT_OSS_120B_MEDIUM',
      ],
    },
  ];
}
```

- [ ] **Step 2: 更新 `crates/cockpit-core/src/modules/account.rs` 的默认分组**

在 `default_auto_switch_groups` 中：
```rust
fn default_auto_switch_groups() -> Vec<AutoSwitchGroupDefinition> {
    vec![
        AutoSwitchGroupDefinition {
            id: "g4_argon".to_string(),
            name: "Gemini 4 Argon".to_string(),
            models: vec![
                "gemini-4-argon".to_string(),
                "gemini-4-argon-medium".to_string(),
                "gemini-4-argon-high".to_string(),
                "argon:weekly".to_string(),
                "gemini-argon-wtu-weekly".to_string(),
            ],
        },
        AutoSwitchGroupDefinition {
            id: "g3_flash".to_string(),
            name: "Gemini Flash".to_string(),
            models: vec![
                "gemini-3.8-flash-high".to_string(),
                "gemini-3.8-flash-medium".to_string(),
                "gemini-3.8-flash-low".to_string(),
                "gemini-3.8-flash".to_string(),
                "gemini-3-flash".to_string(),
                "MODEL_PLACEHOLDER_M318".to_string(),
                "MODEL_PLACEHOLDER_M18".to_string(),
            ],
        },
        AutoSwitchGroupDefinition {
            id: "claude_45".to_string(),
            name: "Claude".to_string(),
            models: vec![
                "claude-opus-5-5-medium".to_string(),
                "claude-opus-5-5-high".to_string(),
                "claude-opus-5-5".to_string(),
                "claude-sonnet-5-5-medium".to_string(),
                "claude-sonnet-5-5-high".to_string(),
                "claude-sonnet-5-5".to_string(),
                "claude-sonnet-4-6".to_string(),
                "claude-opus-4-6-thinking".to_string(),
                "gpt-oss-120b-medium".to_string(),
                "MODEL_PLACEHOLDER_M26".to_string(),
                "MODEL_PLACEHOLDER_M35".to_string(),
                "MODEL_OPENAI_GPT_OSS_120B_MEDIUM".to_string(),
            ],
        },
    ]
}
```

- [ ] **Step 3: 更新 `src-tauri/src/modules/group_settings.rs`**

在 `GroupSettings::default()` 中移除 `g3_pro` 的映射，添加 `g4_argon` 默认项。

- [ ] **Step 4: 运行类型检查验证**

```bash
npm run typecheck
```
Expected: PASS 无类型报错。

- [ ] **Step 5: 提交改动**

```bash
git add src/utils/modelNames.ts src/services/groupService.ts src-tauri/src/modules/group_settings.rs crates/cockpit-core/src/modules/account.rs
git commit -m "feat(antigravity): restructure default groups with dedicated Gemini 4 Argon and remove Gemini Pro"
```

---

### Task 4: 配额解析与 UI 卡片呈现优化（Rust quota.rs & 前端 presentation）

**Files:**
- Modify: `crates/cockpit-core/src/modules/quota.rs`
- Modify: `src/presentation/platformAccountPresentation.ts`
- Test: `src/presentation/antigravityQuotaPresentation.test.ts`

- [ ] **Step 1: 修改 `crates/cockpit-core/src/modules/quota.rs`**

在 `build_quota_data_from_response` 函数（约第 1021 行）：
```rust
if name.contains("gemini") || name.contains("claude") || name.contains("argon") || name.contains("gpt") {
    quota_data.add_model(name, display_name, percentage, reset_time);
}
```

- [ ] **Step 2: 完善 `src/presentation/platformAccountPresentation.ts` 的展示清洗**

确保在输出 `AgQuotaDisplayItem` 时，所有 label 均经过 `name.replace(/\s*\([^)]*\)/g, '').trim()` 处理。

- [ ] **Step 3: 运行并更新 `antigravityQuotaPresentation.test.ts`**

执行：
```bash
node --import tsx src/presentation/antigravityQuotaPresentation.test.ts
```
确保包含 `argon:weekly`、`gemini-weekly`、`claude-weekly` 的用例全部通过。

- [ ] **Step 4: 提交改动**

```bash
git add crates/cockpit-core/src/modules/quota.rs src/presentation/platformAccountPresentation.ts src/presentation/antigravityQuotaPresentation.test.ts
git commit -m "fix(antigravity): broaden quota capture and sanitize card labels"
```

---

### Task 5: 本地代理元数据同步（`models.json`）

**Files:**
- Modify: `sidecars/cockpit-cliproxy/third_party/CLIProxyAPI/internal/registry/models/models.json`

- [ ] **Step 1: 在 `models.json` 的 `antigravity` 数组中追加新模型**

追加 `gemini-4-argon`、`claude-opus-5-5-medium`、`claude-sonnet-5-5-medium`、`gemini-3.8-flash-high`：
```json
{
  "id": "gemini-4-argon",
  "object": "model",
  "owned_by": "antigravity",
  "type": "antigravity",
  "display_name": "Gemini 4 Argon",
  "name": "gemini-4-argon",
  "description": "Gemini 4 Argon",
  "context_length": 1048576,
  "max_completion_tokens": 65536,
  "thinking": {
    "min": 1,
    "max": 65535,
    "dynamic_allowed": true,
    "levels": ["low", "medium", "high"]
  }
},
{
  "id": "claude-opus-5-5-medium",
  "object": "model",
  "owned_by": "antigravity",
  "type": "antigravity",
  "display_name": "Claude Opus 5.5",
  "name": "claude-opus-5-5-medium",
  "description": "Claude Opus 5.5",
  "context_length": 200000,
  "max_completion_tokens": 64000
},
{
  "id": "claude-sonnet-5-5-medium",
  "object": "model",
  "owned_by": "antigravity",
  "type": "antigravity",
  "display_name": "Claude Sonnet 5.5",
  "name": "claude-sonnet-5-5-medium",
  "description": "Claude Sonnet 5.5",
  "context_length": 200000,
  "max_completion_tokens": 64000
}
```

- [ ] **Step 2: 验证 JSON 格式无语法错误**

```bash
node -e "JSON.parse(fs.readFileSync('sidecars/cockpit-cliproxy/third_party/CLIProxyAPI/internal/registry/models/models.json'))"
```
Expected: 无报错退出。

- [ ] **Step 3: 提交改动**

```bash
git add sidecars/cockpit-cliproxy/third_party/CLIProxyAPI/internal/registry/models/models.json
git commit -m "feat(cliproxy): add gemini-4-argon and claude-5.5 models to antigravity registry"
```

---

### Task 6: 整体测试与类型验证（Verification Before Completion）

**Files:**
- All modified files

- [ ] **Step 1: 运行 TypeScript 严格类型检查**

```bash
npm run typecheck
```
Expected: `tsc --noEmit` 成功，退出码 0。

- [ ] **Step 2: 运行前端自动化测试套件**

```bash
node --import tsx src/presentation/antigravityQuotaPresentation.test.ts
```
Expected: 全部测试 PASS。

- [ ] **Step 3: 检查 Git 变更树与提交历史**

```bash
git status
git log -n 5 --oneline
```
Expected: 工作区干净，提交记录清晰规范。
