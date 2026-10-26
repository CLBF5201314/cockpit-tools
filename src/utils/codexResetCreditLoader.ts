import type { CodexResetCreditsSnapshot } from '../types/codex';

/** Share detail requests between account cards and the dashboard for five minutes. */
export function createResetCreditLoader(
  fetchSnapshot: (accountId: string) => Promise<CodexResetCreditsSnapshot>,
  now: () => number = Date.now,
) {
  const cache = new Map<string, {
    count: number;
    expiresAt: number;
    pending: Promise<CodexResetCreditsSnapshot>;
  }>();
  return (accountId: string, count: number) => {
    const cached = cache.get(accountId);
    if (cached?.count === count && cached.expiresAt > now()) return cached.pending;
    const entry = {
      count,
      expiresAt: now() + 5 * 60_000,
      pending: fetchSnapshot(accountId).catch(error => {
        if (cache.get(accountId) === entry) cache.delete(accountId);
        throw error;
      }),
    };
    cache.set(accountId, entry);
    return entry.pending;
  };
}
