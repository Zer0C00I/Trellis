import { call } from '../api.js';
import { banner, card, field, lockHint, moneyButton, withFeedback } from '../components.js';
import { h } from '../dom.js';
import { t, tMsg } from '../i18n.js';

export function render(ctx) {
  const v = ctx.store.vault;
  const root = h('div', { class: 'stack' });

  const payees = h('ul', { class: 'list' });
  if (v.payees.length === 0) payees.append(h('li', { class: 'muted' }, t('people.payees_empty')));
  for (const p of v.payees) {
    payees.append(h('li', {}, h('span', {}, h('strong', {}, p.name), h('small', { class: 'muted' }, ` ${p.address_short}`)), h('span', { class: p.active ? 'ok' : 'muted' }, tMsg(p.status))));
  }
  root.append(card(t('people.payees'), payees));

  if (v.permissions.add_payee) {
    const name = h('input', { type: 'text', maxlength: 16, placeholder: t('people.name_placeholder') });
    const address = h('input', { type: 'text', autocomplete: 'off', spellcheck: 'false', placeholder: t('people.payee_address_placeholder') });
    const out = h('div', { 'aria-live': 'polite' });
    const go = moneyButton(t('people.add_payee_submit'), async () => {
      const r = await withFeedback(go, out, () => call('add_payee', { vault: v.address, name: name.value, address: address.value }));
      if (r.ok) {
        name.value = '';
        address.value = '';
        ctx.notify(r.value);
        await ctx.refresh();
      }
    });
    root.append(card(t('people.add_payee_title'), field(t('people.name_label'), name), field(t('people.address_label'), address, t('people.payee_address_hint')), go, lockHint(), out));
  }

  const guardians = h('ul', { class: 'list' });
  if (v.guardians.length === 0) guardians.append(h('li', { class: 'muted' }, t('people.guardians_empty')));
  for (const g of v.guardians) {
    guardians.append(h('li', {}, h('code', {}, g.address_short), g.is_me ? h('span', { class: 'ok' }, t('people.you')) : null));
  }
  root.append(card(t('people.guardians'), h('p', { class: 'muted' }, t('people.guardians_note')), guardians));

  if (v.permissions.add_guardian) {
    const address = h('input', { type: 'text', autocomplete: 'off', spellcheck: 'false', placeholder: t('people.guardian_address_placeholder') });
    const out = h('div', { 'aria-live': 'polite' });
    const go = moneyButton(t('people.add_guardian_submit'), async () => {
      const r = await withFeedback(go, out, () => call('add_guardian', { vault: v.address, address: address.value }));
      if (r.ok) {
        address.value = '';
        ctx.notify(r.value);
        await ctx.refresh();
      }
    });
    root.append(card(t('people.add_guardian_title'), field(t('people.address_label'), address), go, lockHint(), out));
  } else if (v.is_owner) {
    root.append(banner('info', t('people.cannot_add_guardian')));
  }
  return root;
}
