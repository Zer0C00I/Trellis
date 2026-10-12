import { call } from '../api.js';
import { banner, button, card, field, lockHint, moneyButton, withFeedback } from '../components.js';
import { h } from '../dom.js';
import { currentLocale, t, tMsg } from '../i18n.js';

export function render(ctx) {
  const v = ctx.store.vault;
  if (!v.permissions.send) {
    const why = t(!v.is_owner ? 'send.why_not_owner' : 'send.why_wait');
    return card(t('send.title'), banner('info', why), button(t('common.back'), () => ctx.go('home'), { secondary: true }));
  }

  const ready = v.payees.filter((p) => p.active);
  const chosen = { address: ready[0]?.address };
  const amount = h('input', { type: 'text', inputmode: 'decimal', placeholder: t('common.amount_placeholder') });
  const out = h('div', { 'aria-live': 'polite' });

  const options = ready.map((p, i) =>
    h('label', { class: 'choice' },
      h('input', { type: 'radio', name: 'payee', value: p.address, checked: i === 0, onchange: () => (chosen.address = p.address) }),
      h('span', {}, p.name, h('small', { class: 'muted' }, ` ${p.address_short}`)),
    ),
  );

  const go = moneyButton(t('send.submit'), async () => {
    const r = await withFeedback(go, out, () => call('send_money', { vault: v.address, recipient: chosen.address, amount: amount.value }));
    if (r.ok) {
      amount.value = '';
      ctx.notify(r.value);
      await ctx.refresh();
    }
  });

  const notReady = v.payees.filter((p) => !p.active);
  return h('div', { class: 'stack' },
    card(t('send.to'), h('fieldset', { class: 'choices' }, h('legend', { class: 'sr-only' }, t('send.recipient_legend')), options)),
    card(t('send.how_much'), field(t('common.amount_label'), amount, v.remaining_today ? t('send.left_today_hint', { left: v.remaining_today }) : null), go, lockHint(), out),
    notReady.length
      ? banner('info', t('send.not_ready', { list: notReady.map((p) => `${p.name} (${tMsg(p.status).toLocaleLowerCase(currentLocale())})`).join(', ') }))
      : null,
  );
}
