import { call } from '../api.js';
import { banner, card, field, lockHint, moneyButton, withFeedback } from '../components.js';
import { h } from '../dom.js';

export function render(ctx) {
  const v = ctx.store.vault;
  const root = h('div', { class: 'stack' });

  const payees = h('ul', { class: 'list' });
  if (v.payees.length === 0) payees.append(h('li', { class: 'muted' }, 'Получателей пока нет.'));
  for (const p of v.payees) {
    payees.append(h('li', {}, h('span', {}, h('strong', {}, p.name), h('small', { class: 'muted' }, ` ${p.address_short}`)), h('span', { class: p.active ? 'ok' : 'muted' }, p.status)));
  }
  root.append(card('Получатели', payees));

  if (v.permissions.add_payee) {
    const name = h('input', { type: 'text', maxlength: 16, placeholder: 'Например: Внучка Анна' });
    const address = h('input', { type: 'text', autocomplete: 'off', spellcheck: 'false', placeholder: 'Адрес получателя' });
    const out = h('div', { 'aria-live': 'polite' });
    const go = moneyButton('Добавить получателя', async () => {
      const r = await withFeedback(go, out, () => call('add_payee', { vault: v.address, name: name.value, address: address.value }));
      if (r.ok) {
        name.value = '';
        address.value = '';
        ctx.notify(r.value.message);
        await ctx.refresh();
      }
    });
    root.append(card('Добавить получателя', field('Имя', name), field('Адрес', address, 'Человек присылает вам свой адрес со вкладки «Я».'), go, lockHint(), out));
  }

  const guardians = h('ul', { class: 'list' });
  if (v.guardians.length === 0) guardians.append(h('li', { class: 'muted' }, 'Доверенных лиц пока нет.'));
  for (const g of v.guardians) {
    guardians.append(h('li', {}, h('code', {}, g.address_short), g.is_me ? h('span', { class: 'ok' }, 'это вы') : null));
  }
  root.append(card('Доверенные лица', h('p', { class: 'muted' }, 'Они одобряют переводы, которые больше дневного лимита.'), guardians));

  if (v.permissions.add_guardian) {
    const address = h('input', { type: 'text', autocomplete: 'off', spellcheck: 'false', placeholder: 'Адрес доверенного лица' });
    const out = h('div', { 'aria-live': 'polite' });
    const go = moneyButton('Добавить доверенное лицо', async () => {
      const r = await withFeedback(go, out, () => call('add_guardian', { vault: v.address, address: address.value }));
      if (r.ok) {
        address.value = '';
        ctx.notify(r.value.message);
        await ctx.refresh();
      }
    });
    root.append(card('Добавить доверенное лицо', field('Адрес', address), go, lockHint(), out));
  } else if (v.is_owner) {
    root.append(banner('info', 'Добавить ещё одно доверенное лицо с этого устройства нельзя: нужна подпись уже добавленного.'));
  }
  return root;
}
