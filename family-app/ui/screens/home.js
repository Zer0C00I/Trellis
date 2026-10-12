import { call } from '../api.js';
import { banner, button, card, field, lockHint, moneyButton, rich, withFeedback } from '../components.js';
import { h } from '../dom.js';
import { formatTime } from '../format.js';
import { t, tMsg } from '../i18n.js';

export function render(ctx) {
  const { store } = ctx;
  const v = store.vault;
  const root = h('div', { class: 'stack' });

  const role = t(v.is_owner ? 'home.role_owner' : v.is_guardian ? 'home.role_guardian' : 'home.role_other');
  root.append(
    h('section', { class: 'card hero' },
      h('p', { class: 'muted' }, t(v.is_pool ? 'home.title_pool' : 'home.title_personal')),
      h('p', { class: 'balance', 'aria-label': t('home.balance_label', { balance: v.balance }) }, h('span', { class: 'nowrap' }, v.balance)),
      v.remaining_today
        ? h('p', {}, rich(v.daily_limit ? 'home.left_today_of' : 'home.left_today', { left: v.remaining_today, limit: v.daily_limit }, ['left', 'limit']))
        : null,
      h('p', { class: 'muted' }, role),
    ),
  );

  if (v.permissions.approve) {
    root.append(banner('warn', t('home.approve_waiting', { n: v.pending.length }), button(t('home.view'), () => ctx.go('approvals'))));
  } else if (v.pending.length > 0) {
    root.append(banner('info', t('home.requests_waiting', { n: v.pending.length })));
  }

  const actions = h('div', { class: 'row' });
  if (v.permissions.send) actions.append(button(t('home.send'), () => ctx.go('send')));
  actions.append(button(t('home.deposit'), () => toggleDeposit(), { secondary: v.permissions.send }));
  root.append(actions);

  const depositBox = h('div');
  root.append(depositBox);

  function toggleDeposit() {
    if (depositBox.childElementCount) return depositBox.replaceChildren();
    const amount = h('input', { type: 'text', inputmode: 'decimal', placeholder: t('common.amount_placeholder') });
    const out = h('div', { 'aria-live': 'polite' });
    const go = moneyButton(t('deposit.submit'), async () => {
      const r = await withFeedback(go, out, () => call('deposit', { vault: v.address, amount: amount.value }));
      if (r.ok) {
        amount.value = '';
        ctx.notify(r.value);
        await ctx.refresh();
      }
    });
    depositBox.append(card(t('deposit.title'), field(t('common.amount_label'), amount, t('deposit.hint')), go, lockHint(), out));
  }

  const list = h('ul', { class: 'list' });
  if (store.history.length === 0) list.append(h('li', { class: 'muted' }, t('home.history_empty')));
  for (const item of store.history.slice(0, 10)) {
    list.append(h('li', {}, h('span', {}, tMsg(item)), h('span', { class: 'muted' }, formatTime(item.time))));
  }
  root.append(card(t('home.history_title'), list));
  return root;
}
