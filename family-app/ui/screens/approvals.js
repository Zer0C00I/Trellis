import { call } from '../api.js';
import { banner, card, lockHint, moneyButton, withFeedback } from '../components.js';
import { h } from '../dom.js';
import { formatTime } from '../format.js';
import { t } from '../i18n.js';

export function render(ctx) {
  const v = ctx.store.vault;
  if (v.pending.length === 0) return card(t('approvals.title'), banner('info', t('approvals.none')));

  const items = v.pending.map((r) => {
    const out = h('div', { 'aria-live': 'polite' });
    const approve = moneyButton(t('approvals.approve'), async () => {
      const result = await withFeedback(approve, out, () => call('approve_request', { vault: v.address, nonce: r.nonce }));
      if (result.ok) {
        ctx.notify(result.value);
        await ctx.refresh();
      }
    }, { disabled: !v.permissions.approve });
    return card(
      t('approvals.transfer', { amount: r.amount }),
      h('p', {}, `${t('approvals.recipient')} `, h('code', {}, r.to_short)),
      h('p', { class: 'muted' }, t('approvals.requested', { time: formatTime(r.requested_at) })),
      v.permissions.approve ? [approve, lockHint()] : banner('info', t('approvals.only_guardian')),
      out,
    );
  });
  return h('div', { class: 'stack' }, ...items);
}
