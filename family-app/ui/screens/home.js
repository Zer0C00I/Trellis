import { call } from '../api.js';
import { banner, button, card, field, lockHint, moneyButton, withFeedback } from '../components.js';
import { h } from '../dom.js';
import { formatTime } from '../format.js';

export function render(ctx) {
  const { store } = ctx;
  const v = store.vault;
  const root = h('div', { class: 'stack' });

  const role = v.is_owner ? 'Вы владелец счёта' : v.is_guardian ? 'Вы доверенное лицо' : 'Вы можете пополнять счёт';
  root.append(
    h('section', { class: 'card hero' },
      h('p', { class: 'muted' }, v.title),
      h('p', { class: 'balance', 'aria-label': `Баланс: ${v.balance}` }, h('span', { class: 'nowrap' }, v.balance)),
      v.remaining_today ? h('p', {}, 'Сегодня можно отправить ещё ', h('span', { class: 'nowrap' }, v.remaining_today), v.daily_limit ? [' из ', h('span', { class: 'nowrap' }, v.daily_limit)] : null) : null,
      h('p', { class: 'muted' }, role),
    ),
  );

  if (v.permissions.approve) {
    root.append(banner('warn', `Ждут вашего одобрения: ${v.pending.length}`, button('Посмотреть', () => ctx.go('approvals'))));
  } else if (v.pending.length > 0) {
    root.append(banner('info', `Запросов на перевод, которые ждут одобрения доверенного лица: ${v.pending.length}`));
  }

  const actions = h('div', { class: 'row' });
  if (v.permissions.send) actions.append(button('Отправить', () => ctx.go('send')));
  actions.append(button('Пополнить', () => toggleDeposit(), { secondary: v.permissions.send }));
  root.append(actions);

  const depositBox = h('div');
  root.append(depositBox);

  function toggleDeposit() {
    if (depositBox.childElementCount) return depositBox.replaceChildren();
    const amount = h('input', { type: 'text', inputmode: 'decimal', placeholder: 'Например: 25 или 2.50' });
    const out = h('div', { 'aria-live': 'polite' });
    const go = moneyButton('Пополнить', async () => {
      const r = await withFeedback(go, out, () => call('deposit', { vault: v.address, amount: amount.value }));
      if (r.ok) {
        amount.value = '';
        ctx.notify(r.value.message);
        await ctx.refresh();
      }
    });
    depositBox.append(card('Пополнить счёт', field('Сумма (USDC)', amount, 'Деньги берутся с вашего адреса. Тестовые USDC можно получить на faucet.circle.com.'), go, lockHint(), out));
  }

  const list = h('ul', { class: 'list' });
  if (store.history.length === 0) list.append(h('li', { class: 'muted' }, 'Пока ничего не происходило.'));
  for (const item of store.history.slice(0, 10)) {
    list.append(h('li', {}, h('span', {}, item.text), h('span', { class: 'muted' }, formatTime(item.time))));
  }
  root.append(card('Последние операции', list));
  return root;
}
