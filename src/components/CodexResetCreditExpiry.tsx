import { useEffect, useState } from 'react';
import { AlertTriangle, Clock } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { CodexQuota } from '../types/codex';
import { getResetCreditExpiry } from '../utils/codexResetCreditExpiry';
import './CodexResetCreditExpiry.css';

interface Props {
  accountId: string;
  quota: CodexQuota | undefined;
  disabled: boolean;
  onClick: () => void;
}

export function CodexResetCreditExpiry({ quota, disabled, onClick }: Props) {
  const { t, i18n } = useTranslation();
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  const available = (quota?.reset_credits_available ?? 0) > 0;
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
  // The ordinary quota refresh already supplies these fields. Opening account
  // cards must not start another network request per visible card.
  const expiry = getResetCreditExpiry({
    available_count: quota?.reset_credits_available,
    credits: quota?.reset_credits ?? [],
    next_expires_at: quota?.reset_credits_next_expires_at,
  }, now);
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
