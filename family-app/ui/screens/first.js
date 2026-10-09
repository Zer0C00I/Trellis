// First launch: this phone has no vault yet. Two ways in:
//  1. be added by a kiosk during ITS first-run setup (type what the kiosk
//     shows), then wait until the chain says so;
//  2. open a family account of your own.
import { call } from '../api.js';
import { button, card, errorBanner, field, banner, withFeedback } from '../components.js';
import { h } from '../dom.js';
import { classifyPairingInput } from '../input.js';

export function render(ctx) {
  const { store } = ctx;
  const root = h('div', { class: 'stack' });
  let waiting = false;
  let timer = null;

  const stopWaiting = () => clearInterval(timer);

  function draw() {
    root.replaceChildren(
      h('h1', {}, 'Добро пожаловать'),
      h('p', { class: 'lead' }, 'Это приложение для близких: оно помогает следить за счётом родственника и одобрять переводы.'),
      waiting ? waitingCard() : pairCard(),
      openCard(),
      addressCard(),
    );
  }

  function pairCard() {
    const name = h('input', { type: 'text', maxlength: 24, autocomplete: 'name', placeholder: 'Например: Внук Саша' });
    const code = h('input', { type: 'text', inputmode: 'text', autocomplete: 'off', placeholder: 'fvpair1:… или 6 цифр' });
    const out = h('div', { 'aria-live': 'polite' });
    const go = button('Передать устройству', async () => {
      const input = classifyPairingInput(code.value);
      if (!name.value.trim()) return out.replaceChildren(errorBanner({ kind: 'invalid', message: 'Напишите своё имя: так вас увидит родственник на экране.' }));
      if (input.kind === 'empty' || input.kind === 'unknown') {
        return out.replaceChildren(errorBanner({ kind: 'invalid', message: 'Введите текст QR-кода с экрана устройства или 6 цифр под ним.' }));
      }
      const r = await withFeedback(go, out, () =>
        input.kind === 'pin'
          ? call('pair_with_pin', { pin: input.value, name: name.value.trim() })
          : call('pair_with_qr', { qrText: input.value, name: name.value.trim() }),
      );
      if (!r.ok) return;
      // The kiosk accepted the code, but it may still fail to register us on
      // the chain: keep looking until a vault actually shows up.
      waiting = true;
      draw();
      timer = setInterval(() => ctx.refresh({ background: true }), 5000);
    });
    return card(
      'Меня добавляет родственник',
      h('p', {}, 'Попросите включить настройку на устройстве родственника. На его экране появится QR-код и 6 цифр. Станьте рядом, включите Bluetooth.'),
      field('Ваше имя', name, 'Родственник увидит его на экране устройства.'),
      field('Код с экрана', code, 'Текст QR-кода или 6 цифр под ним.'),
      go,
      out,
    );
  }

  function waitingCard() {
    return card(
      'Ждём, пока вас добавят',
      banner('info', 'Устройство приняло ваш код. Оно записывает вас в список, это занимает около минуты. Экран обновится сам.'),
      button('Проверить сейчас', () => ctx.refresh(), { secondary: true }),
    );
  }

  function openCard() {
    const limit = h('input', { type: 'text', inputmode: 'decimal', placeholder: 'Например: 100' });
    const out = h('div', { 'aria-live': 'polite' });
    const go = button('Открыть семейный счёт', async () => {
      const r = await withFeedback(go, out, () => call('open_family_account', { dailyLimit: limit.value }));
      if (r.ok) await ctx.refresh();
    });
    return card(
      'Или открыть свой семейный счёт',
      h('p', {}, 'Общий счёт семьи в долларах (USDC). Чтобы тратить с него, нужны получатели и доверенные лица.'),
      field('Сколько можно отправить за день без одобрения (USDC)', limit),
      go,
      out,
    );
  }

  // A brand-new key has no SOL, and every action costs a small fee, so the
  // address and the Devnet faucet are reachable before any vault exists.
  function addressCard() {
    const out = h('div', { 'aria-live': 'polite' });
    const faucet = button(
      'Получить тестовые SOL',
      async () => {
        const r = await withFeedback(faucet, out, () => call('get_test_sol'));
        if (r.ok) {
          ctx.notify('Готово: SOL пришли на ваш адрес.');
          await ctx.refresh();
        }
      },
      { secondary: true },
    );
    return card(
      'Ваш адрес и комиссии',
      h('p', {}, 'Каждое действие стоит совсем немного SOL. Родственнику этот адрес нужен, чтобы вас добавить.'),
      h('code', { class: 'address' }, store.info?.address ?? ''),
      h('p', {}, `На адресе: ${store.wallet?.sol ?? '…'} SOL`),
      store.wallet?.low_sol ? banner('warn', 'SOL почти нет. Получите тестовые SOL, иначе счёт не открыть.') : null,
      faucet,
      out,
    );
  }

  draw();
  ctx.onLeave(stopWaiting);
  return root;
}
