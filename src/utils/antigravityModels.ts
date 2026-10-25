import type { Account, ModelQuota, QuotaData } from '../types/account';

export interface AntigravityModelOption {
  id: string;
  displayName: string;
  modelConstant?: string | null;
  recommended?: boolean | null;
}

interface ModelIdentity {
  id?: string | null;
  modelConstant?: string | null;
  displayName?: string | null;
}

interface CanonicalModel {
  id: string;
  displayName: string;
  modelConstant: string | null;
  aliases: string[];
}

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

export const AUTH_RECOMMENDED_LABELS = CANONICAL_MODELS.map((item) => item.displayName);
export const AUTH_RECOMMENDED_MODEL_IDS = CANONICAL_MODELS.map((item) => item.modelConstant);

const normalize = (value?: string | null) =>
  (value || '')
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]/g, '');

const buildAliasMap = () => {
  const map = new Map<string, CanonicalModel>();
  CANONICAL_MODELS.forEach((item) => {
    [item.id, item.modelConstant, item.displayName, ...item.aliases].forEach((value) => {
      const key = normalize(value);
      if (!key || map.has(key)) return;
      map.set(key, item);
    });
  });
  return map;
};

const CANONICAL_BY_ALIAS = buildAliasMap();
const CANONICAL_RANK = new Map<string, number>(
  CANONICAL_MODELS.map((item, index) => [item.id, index]),
);

const resolveCanonicalModel = (identity: ModelIdentity): CanonicalModel | undefined => {
  const keys = [identity.id, identity.modelConstant, identity.displayName]
    .map((value) => normalize(value))
    .filter((value) => value.length > 0);
  for (const key of keys) {
    const found = CANONICAL_BY_ALIAS.get(key);
    if (found) return found;
  }
  return undefined;
};

export const getAntigravityModelDisplayName = (value: string): string => {
  const found = resolveCanonicalModel({ id: value, modelConstant: value, displayName: value });
  const raw = found?.displayName || value || '';
  return raw.replace(/\s*\([^)]*\)/g, '').trim();
};

export const filterAntigravityModelOptions = (
  models: AntigravityModelOption[],
  _options?: {
    allowedModelKeys?: Iterable<string>;
    includeNonRecommended?: boolean;
  },
): AntigravityModelOption[] => {
  if (!Array.isArray(models) || models.length === 0) {
    return [];
  }

  const result: AntigravityModelOption[] = [];
  const seen = new Set<string>();

  models.forEach((model) => {
    const canonical = resolveCanonicalModel({
      id: model.id,
      modelConstant: model.modelConstant,
      displayName: model.displayName,
    });
    if (!canonical) return;
    if (seen.has(canonical.id)) return;
    seen.add(canonical.id);
    result.push({
      id: canonical.id,
      displayName: (model.displayName || '').trim() || canonical.displayName,
      modelConstant: model.modelConstant || canonical.modelConstant,
      recommended: true,
    });
  });

  return result;
};

export const collectAntigravityQuotaModelKeys = (accounts: Account[]): string[] => {
  const keys: string[] = [];
  const seen = new Set<string>();
  accounts.forEach((account) => {
    (account.quota?.models || []).forEach((model) => {
      const key = normalize(model.name);
      if (!key || seen.has(key)) return;
      seen.add(key);
      keys.push(model.name);
    });
  });
  return keys;
};

export const buildAntigravityFallbackModelOptions = (_accounts: Account[]): AntigravityModelOption[] =>
  [];

export const sortAntigravityQuotaModels = (models: ModelQuota[]): ModelQuota[] => {
  if (!Array.isArray(models) || models.length === 0) {
    return [];
  }

  const picked: Array<{ rank: number; model: ModelQuota }> = [];
  const seen = new Set<string>();

  models.forEach((model) => {
    const canonical = resolveCanonicalModel({
      id: model.name,
      modelConstant: model.name,
      displayName: model.display_name || model.name,
    });
    if (!canonical) return;
    if (seen.has(canonical.id)) return;
    seen.add(canonical.id);
    picked.push({
      rank: CANONICAL_RANK.get(canonical.id) ?? Number.MAX_SAFE_INTEGER,
      model,
    });
  });

  return picked
    .sort((a, b) => a.rank - b.rank)
    .map((item) => item.model);
};

export const getAntigravityDisplayModelsFromQuota = (quota?: QuotaData): ModelQuota[] => {
  if (!quota?.models?.length) {
    return [];
  }
  return sortAntigravityQuotaModels(quota.models);
};
