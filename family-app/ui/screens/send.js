import { call } from '../api.js';
import { banner, button, card, field, lockHint, moneyButton, withFeedback } from '../components.js';
import { h } from '../dom.js';

export function render(ctx) {
  const v = ctx.store.vault;
  if (!v.permissions.send) {
    const why = !v.is_owner
      ? 'Отправлять деньги с этого счёта может только его владелец.'
      : 'Пока некому отправлять: нового получателя можно использовать через 48 часов после добавления.';
    return card('Отправить', banner('info', why), button('Назад', () => ctx.go('home'), { secondary: true }));
  }

  const ready = v.payees.filter((p) => p.active);
  const chosen = { address: ready[0]?.address };
  const amount = h('input', { type: 'text', inputmode: 'decimal', placeholder: 'Например: 25 или 2.50' });
  const out = h('div', { 'aria-live': 'polite' });

  const options = ready.map((p, i) =>
    h('label', { class: 'choice' },
      h('input', { type: 'radio', name: 'payee', value: p.address, checked: i === 0, onchange: () => (chosen.address = p.address) }),
      h('span', {}, p.name, h('small', { class: 'muted' }, ` ${p.address_short}`)),
    ),
  );

  const go = moneyButton('Отправить', async () => {
    const r = await withFeedback(go, out, () => call('send_money', { vault: v.address, recipient: chosen.address, amount: amount.value }));
    if (r.ok) {
      amount.value = '';
      ctx.notify(r.value.message);
      await ctx.refresh();
    }
  });

  const notReady = v.payees.filter((p) => !p.active);
  return h('div', { class: 'stack' },
    card('Кому отправить', h('fieldset', { class: 'choices' }, h('legend', { class: 'sr-only' }, 'Получатель'), options)),
    card('Сколько', field('Сумма (USDC)', amount, v.remaining_today ? `Сегодня без одобрения можно ещё ${v.remaining_today}.` : null), go, lockHint(), out),
    notReady.length ? banner('info', `Ещё нельзя использовать: ${notReady.map((p) => `${p.name} (${p.status.toLowerCase()})`).join(', ')}.`) : null,
  );
}
