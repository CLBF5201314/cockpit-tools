import assert from 'node:assert/strict';
import test from 'node:test';
import { loadHookModule, settlePromises } from '../../tests/helpers/reactHookHarness';
import { createResetCreditLoader } from '../utils/codexResetCreditLoader';
import { getResetCreditExpiry } from '../utils/codexResetCreditExpiry';

test('cards load missing expiry details, retain known quota data, and never show another account’s expiry', async () => {
  const requests: string[] = [];
  const h = loadHookModule(new URL('./CodexResetCreditExpiry.tsx', import.meta.url), {
    'react-i18next': { useTranslation: () => ({
      t: (key: string) => key, i18n: { language: 'zh-CN' },
    }) },
    '../utils/codexResetCreditLoader': { createResetCreditLoader },
    '../utils/codexResetCreditExpiry': { getResetCreditExpiry },
    '../services/codexService': { getCodexResetCredits: async (id: string) => {
      requests.push(id);
      if (id === 'b') throw new Error('offline');
      return { available_count: 1, credits: [], next_expires_at: Date.now() / 1000 + 3600 };
    } },
  }, { window: { setInterval: () => 1, clearInterval() {}, addEventListener() {}, removeEventListener() {} } });
  let props: any = { accountId: 'a', quota: { reset_credits_available: 1 }, disabled: false, onClick() {} };
  const render = () => h.exports.CodexResetCreditExpiry(props);
  assert.match(h.render(render).props.title, /TimeUnknown/);
  await settlePromises();
  assert.match(h.flush().props.title, /ExpiryUrgent/);
  assert.deepEqual(requests, ['a']);

  props = { ...props, accountId: 'b' };
  assert.match(h.render(render).props.title, /TimeUnknown/);
  await settlePromises();
  assert.match(h.flush().props.title, /TimeUnknown/);

  props = { ...props, accountId: 'c', quota: { reset_credits_available: 1,
    reset_credits_next_expires_at: Date.now() / 1000 + 86400 } };
  assert.match(h.render(render).props.title, /ExpiryNormal/);
  assert.deepEqual(requests, ['a', 'b']);
  props = { ...props, quota: { reset_credits_available: 0 } };
  assert.equal(h.render(render), null);
  h.unmount();
});
