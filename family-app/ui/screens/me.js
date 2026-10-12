import { call } from '../api.js';
import { banner, button, card, withFeedback } from '../components.js';
import { h } from '../dom.js';
import { t } from '../i18n.js';

export function render(ctx) {
  const { store } = ctx;
  const out = h('div', { 'aria-live': 'polite' });
  const copyOut = h('span', { class: 'muted' });

  const copy = button(t('me.copy'), async () => {
    try {
      await navigator.clipboard.writeText(store.info.address);
      copyOut.textContent = ` ${t('me.copied')}`;
    } catch {
      copyOut.textContent = ` ${t('me.copy_failed')}`;
    }
  }, { secondary: true });

  const faucet = button(t('faucet.button'), async () => {
    const r = await withFeedback(faucet, out, () => call('get_test_sol'));
    if (r.ok) {
      ctx.notify({ key: 'faucet.done' });
      await ctx.refresh();
    }
  });

  return h('div', { class: 'stack' },
    card(t('me.address_title'), h('p', {}, t('me.address_text')), h('code', { class: 'address' }, store.info.address), h('div', { class: 'row' }, copy, copyOut)),
    card(t('me.fees_title'), h('p', {}, t('address.on_address', { sol: store.wallet?.sol ?? '…' })), store.wallet?.low_sol ? banner('warn', t('me.low_sol')) : null, faucet, out),
    card(t('me.about_title'), h('p', { class: 'muted' }, t('me.about', { network: store.info.network === 'devnet' ? t('me.network_devnet') : store.info.network }))),
  );
}
