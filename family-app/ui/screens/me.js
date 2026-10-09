import { call } from '../api.js';
import { banner, button, card, withFeedback } from '../components.js';
import { h } from '../dom.js';

export function render(ctx) {
  const { store } = ctx;
  const out = h('div', { 'aria-live': 'polite' });
  const copyOut = h('span', { class: 'muted' });

  const copy = button('Скопировать адрес', async () => {
    try {
      await navigator.clipboard.writeText(store.info.address);
      copyOut.textContent = ' Скопировано';
    } catch {
      copyOut.textContent = ' Не удалось скопировать: выделите адрес и скопируйте вручную.';
    }
  }, { secondary: true });

  const faucet = button('Получить тестовые SOL', async () => {
    const r = await withFeedback(faucet, out, () => call('get_test_sol'));
    if (r.ok) {
      ctx.notify('Готово: SOL пришли на ваш адрес.');
      await ctx.refresh();
    }
  });

  return h('div', { class: 'stack' },
    card('Ваш адрес', h('p', {}, 'Так вас находят в сети. Этот адрес можно отправлять родственникам, чтобы они добавили вас получателем.'), h('code', { class: 'address' }, store.info.address), h('div', { class: 'row' }, copy, copyOut)),
    card('Комиссии', h('p', {}, `На адресе: ${store.wallet?.sol ?? '…'} SOL`), store.wallet?.low_sol ? banner('warn', 'SOL почти нет. Без них нельзя ничего отправить или одобрить.') : null, faucet, out),
    card('О приложении', h('p', { class: 'muted' }, `Сеть: ${store.info.network === 'devnet' ? 'тестовая (Devnet)' : store.info.network}. Это учебный прототип без аудита: используйте только тестовые деньги.`)),
  );
}
