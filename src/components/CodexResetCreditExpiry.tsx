import { useEffect, useState } from 'react';
import { AlertTriangle, Clock } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { CodexQuota, CodexResetCreditsSnapshot } from '../types/codex';
import { getResetCreditExpiry } from '../utils/codexResetCreditExpiry';
import { createResetCreditLoader } from '../utils/codexResetCreditLoader';
import { getCodexResetCredits } from '../services/codexService';
import './CodexResetCreditExpiry.css';

const loadResetCredits = createResetCreditLoader(getCodexResetCredits);

interface Props {
  accountId: string;
  quota: CodexQuota | undefined;
  disabled: boolean;
  onClick: () => void;
}

export function CodexResetCreditExpiry({ accountId, quota, disabled, onClick }: Props) {
  const { t, i18n } = useTranslation();
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  const available = (quota?.reset_credits_available ?? 0) > 0;
  const count = quota?.reset_credits_available ?? 0;
  const [details, setDetails] = useState<{
    accountId: string; count: number; snapshot: CodexResetCreditsSnapshot;
  } | null>(null);
  const quotaHasExpiry = Boolean(quota?.reset_credits_next_expires_at)
    || Boolean(quota?.reset_credits?.some(credit => credit.expires_at));
  useEffect(() => {
    if (!available || quotaHasExpiry) return;
    let active = true;
    // The usage endpoint only reports the count. Fetch missing expiry details
    // separately, sharing requests across views without delaying account cards.
    void loadResetCredits(accountId, count).then(snapshot => {
      if (active) setDetails({ accountId, count, snapshot });
    }).catch(() => {
      // The existing details action remains available for a manual retry.
    });
    return () => { active = false; };
  }, [accountId, count, available, quotaHasExpiry]);
  useEffect(() => {
    if (!available) return;
    const tick = () => setNow(Math.floor(Date.now() / 1000));
    tick();
    const timer = window.setInterval(tick, 30_000);
    window.addEventListener('focus', tick);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener('focus', tick);
    };
  }, [available]);

  if (!available) return null;
  const snapshot = !quotaHasExpiry && details?.accountId === accountId && details.count === count
    ? details.snapshot : {
    available_count: quota?.reset_credits_available,
    credits: quota?.reset_credits ?? [],
    next_expires_at: quota?.reset_credits_next_expires_at,
  };
  const expiry = getResetCreditExpiry(snapshot, now);
  const absolute = expiry ? new Date(expiry.expiresAt * 1000).toLocaleString(i18n.language, {
    month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false,
  }) : '';
  const minutes = expiry ? Math.max(1, Math.ceil(expiry.remaining / 60)) : 0;
  const duration = minutes >= 60
    ? t('codex.quota.resetCreditExpiryHours', { hours: Math.floor(minutes / 60), minutes: minutes % 60 })
    : t('codex.quota.resetCreditExpiryMinutes', { minutes });
  const text = !expiry ? t('codex.quota.resetCreditTimeUnknown')
    : expiry.expired ? t('codex.quota.resetCreditExpiryExpired')
    : t(expiry.urgent ? 'codex.quota.resetCreditExpiryUrgent' : 'codex.quota.resetCreditExpiryNormal', {
      time: absolute, duration,
    });
  return (
    <button type="button" onClick={onClick} disabled={disabled}
      className={`codex-reset-credit-expiry ${expiry?.urgent || expiry?.expired ? 'is-urgent' : ''}`}
      title={text}>
      {expiry?.urgent || expiry?.expired ? <AlertTriangle size={13} /> : <Clock size={13} />}
      <span>{text}</span>
    </button>
  );
}
