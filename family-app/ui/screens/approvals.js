import { call } from '../api.js';
import { banner, button, card, withFeedback } from '../components.js';
import { h } from '../dom.js';
import { formatTime } from '../format.js';

export function render(ctx) {
  const v = ctx.store.vault;
  if (v.pending.length === 0) return card('Одобрения', banner('info', 'Сейчас нет переводов, которые ждут одобрения.'));

  const items = v.pending.map((r) => {
    const out = h('div', { 'aria-live': 'polite' });
    const approve = button('Одобрить', async () => {
      const result = await withFeedback(approve, out, () => call('approve_request', { vault: v.address, nonce: r.nonce }));
      if (result.ok) {
        ctx.notify(result.value.message);
        await ctx.refresh();
      }
    }, { disabled: !v.permissions.approve });
    return card(
      `Перевод ${r.amount}`,
      h('p', {}, 'Получатель: ', h('code', {}, r.to_short)),
      h('p', { class: 'muted' }, `Запрошен: ${formatTime(r.requested_at)}`),
      v.permissions.approve ? approve : banner('info', 'Одобрить может только доверенное лицо.'),
      out,
    );
  });
  return h('div', { class: 'stack' }, ...items);
}
