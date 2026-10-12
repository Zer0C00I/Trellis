// First launch: this phone has no vault yet. Two ways in:
//  1. be added by a kiosk during ITS first-run setup (type what the kiosk
//     shows), then wait until the chain says so;
//  2. open a family account of your own.
import { call, isMock } from '../api.js';
import { button, card, errorBanner, field, banner, lockHint, moneyButton, withFeedback } from '../components.js';
import { h } from '../dom.js';
import { t } from '../i18n.js';
import { classifyPairingInput } from '../input.js';

export function render(ctx) {
  const { store } = ctx;
  const root = h('div', { class: 'stack' });
  function draw() {
    root.replaceChildren(
      isMock ? h('span', { class: 'tag' }, t('header.demo')) : null,
      h('h1', {}, t('first.title')),
      h('p', { class: 'lead' }, t('first.lead')),
      store.waiting ? waitingCard() : pairCard(),
      openCard(),
      addressCard(),
    );
  }

  function pairCard() {
    const name = h('input', { type: 'text', maxlength: 24, autocomplete: 'name', placeholder: t('pair.name_placeholder') });
    const code = h('input', { type: 'text', inputmode: 'text', autocomplete: 'off', placeholder: t('pair.code_placeholder') });
    const out = h('div', { 'aria-live': 'polite' });
    const go = button(t('pair.submit'), async () => {
      const input = classifyPairingInput(code.value);
      if (!name.value.trim()) return out.replaceChildren(errorBanner({ kind: 'invalid', key: 'pair.need_name', params: {} }));
      if (input.kind === 'empty' || input.kind === 'unknown') {
        return out.replaceChildren(errorBanner({ kind: 'invalid', key: 'pair.need_code', params: {} }));
      }
      const r = await withFeedback(go, out, () =>
        input.kind === 'pin'
          ? call('pair_with_pin', { pin: input.value, name: name.value.trim() })
          : call('pair_with_qr', { qrText: input.value, name: name.value.trim() }),
      );
      if (!r.ok) return;
      // The kiosk accepted the code, but it may still fail to register us on
      // the chain: keep looking until a vault actually shows up.
      // The shell keeps polling (see app.js) and remembers this across re-renders.
      ctx.setWaiting(true);
    });
    return card(
      t('pair.title'),
      h('p', {}, t('pair.text')),
      store.pairHint ? h('p', { class: 'hint' }, t('pair.hint')) : null,
      field(t('pair.name_label'), name, t('pair.name_hint')),
      field(t('pair.code_label'), code, t('pair.code_hint')),
      go,
      out,
    );
  }

  function waitingCard() {
    return card(
      t('waiting.title'),
      banner('info', t('waiting.text')),
      h('div', { class: 'btn-group' },
        button(t('waiting.check'), () => ctx.refresh(), { secondary: true }),
        button(t('waiting.retry'), () => ctx.setWaiting(false), { secondary: true }),
      ),
    );
  }

  function openCard() {
    const limit = h('input', { type: 'text', inputmode: 'decimal', placeholder: t('open.limit_placeholder') });
    const out = h('div', { 'aria-live': 'polite' });
    const go = moneyButton(t('open.submit'), async () => {
      const r = await withFeedback(go, out, () => call('open_family_account', { dailyLimit: limit.value }));
      if (r.ok) await ctx.refresh();
    });
    return card(
      t('open.title'),
      h('p', {}, t('open.text')),
      field(t('open.limit_label'), limit),
      go,
      lockHint(),
      out,
    );
  }

  // A brand-new key has no SOL, and every action costs a small fee, so the
  // address and the Devnet faucet are reachable before any vault exists.
  function addressCard() {
    const out = h('div', { 'aria-live': 'polite' });
    const faucet = button(
      t('faucet.button'),
      async () => {
        const r = await withFeedback(faucet, out, () => call('get_test_sol'));
        if (r.ok) {
          ctx.notify({ key: 'faucet.done' });
          await ctx.refresh();
        }
      },
      { secondary: true },
    );
    return card(
      t('address.title'),
      h('p', {}, t('address.text')),
      h('code', { class: 'address' }, store.info?.address ?? ''),
      h('p', {}, t('address.on_address', { sol: store.wallet?.sol ?? '…' })),
      store.wallet?.low_sol ? banner('warn', t('address.low_sol')) : null,
      faucet,
      out,
    );
  }

  draw();
  return root;
}
