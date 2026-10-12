export const APP_URL = '?mock=demo&lang=en';
export const KIOSK_SCREENS = ['wallet', 'family', 'amount', 'confirm', 'ok', 'fail', 'setup', 'wait'];

const ANNA = 'Granddaughter Anna';
const OTHER_FAMILY = 'Cw5RnE8yMq2KdLtV7XbZa3JhUp9GfoSi1TmBvNeY4AkD'; // a relative’s family vault where the user is a guardian (mock)

// kiosk is a list of beats: { at: seconds into the scene, screen?, name?, amount?, tap?: { x, y } as fractions of the screen }.
// A beat with `tap` shows a finger tap 0.5 s before `at` on the screen then showing; without `screen` it only taps.
// link.dir is spatial: 'right' = from the kiosk (left) to the app (right), 'left' = from the app to the kiosk.
export const scenes = [
  {
    id: 'ready', seconds: 7,
    link: null,
    caption: 'Trellis: the elder uses a simple kiosk. A relative opens the family app.',
    kiosk: [{ at: 0, screen: 'wallet' }],
    app: [{ do: 'wait', key: 'pair.title' }],
  },
  {
    id: 'pairing', seconds: 9,
    link: { via: 'Bluetooth LE', dir: 'left', label: '6-digit code 123456 goes from the phone to the kiosk' },
    caption: 'Pairing: the kiosk shows a one-time code, the relative types it.',
    kiosk: [{ at: 0, screen: 'setup' }],
    app: [
      { do: 'fill', selector: 'input[autocomplete=name]', value: 'Sasha' },
      { do: 'fill', selector: 'input[placeholder^="fvpair1"]', value: '123456' },
    ],
  },
  {
    id: 'waiting', seconds: 7,
    link: { via: 'Solana devnet', dir: 'right', label: 'the kiosk adds the relative to its vault on-chain' },
    caption: 'Handed over to the device. The app waits to be added.',
    kiosk: [{ at: 0, screen: 'setup' }],
    app: [{ do: 'click', key: 'pair.submit' }, { do: 'wait', key: 'waiting.title' }],
  },
  {
    id: 'family', seconds: 9,
    link: { via: 'Solana devnet', dir: 'both', label: 'both screens read the same vault: 340 USDC' },
    caption: 'The family vault: 340 USDC, and at most 30 USDC can leave per day. The Solana program enforces it.',
    kiosk: [{ at: 0, screen: 'wallet' }, { at: 3, tap: { x: 0.85, y: 0.11 }, screen: 'family' }],
    app: [{ do: 'wait', key: 'home.title_pool' }],
  },
  {
    id: 'within-limit', seconds: 9,
    link: { via: 'Solana devnet', dir: 'left', label: 'send_money 10 USDC, confirmed on-chain' },
    caption: 'Within the limit, 10 USDC goes through at once, and the kiosk shows the new balance.',
    kiosk: [{ at: 0, screen: 'family' }, { at: 6, screen: 'family', familyBalance: '330 USDC' }],
    app: [
      { do: 'click', key: 'tabs.send' },
      { do: 'fill', selector: 'input[inputmode=decimal]', value: '10' },
      { do: 'click', key: 'send.submit' },
      { do: 'wait', key: 'notice.sent' },
    ],
  },
  {
    id: 'over-limit', seconds: 9,
    link: { via: 'Solana devnet', dir: 'left', label: '45 USDC is above the daily limit: filed as a request, not paid' },
    caption: 'Above the limit, 45 USDC is not paid. It becomes a request that a guardian must approve.',
    kiosk: [{ at: 0, screen: 'family', familyBalance: '330 USDC' }],
    app: [
      { do: 'fill', selector: 'input[inputmode=decimal]', value: '45' },
      { do: 'click', key: 'send.submit' },
      { do: 'wait', key: 'notice.request_filed' },
    ],
  },
  {
    id: 'cooldown', seconds: 9,
    link: { via: 'Solana devnet', dir: 'both', label: 'add_payee: a new payee is usable only after 48 hours' },
    caption: 'A newly added person waits 48 hours before they can be paid, so a stolen key cannot cash out.',
    kiosk: [{ at: 0, screen: 'family', familyBalance: '330 USDC' }],
    app: [{ do: 'click', key: 'tabs.people' }, { do: 'wait', key: 'people.payees' }],
  },
  {
    id: 'guardian', seconds: 9,
    link: { via: 'Solana devnet', dir: 'both', label: 'each device reads the chain on its own' },
    caption: 'The same app works as a guardian: in another family’s vault, requests wait for your approval.',
    kiosk: [{ at: 0, screen: 'family', familyBalance: '330 USDC' }],
    app: [
      { do: 'select', selector: 'select[aria-label]', value: OTHER_FAMILY },
      { do: 'click', key: 'tabs.home' },
      { do: 'wait', key: 'home.approve_waiting', params: { n: 1 } },
    ],
  },
  {
    id: 'approve', seconds: 10,
    link: { via: 'Solana devnet', dir: 'right', label: 'approve_request is signed by the guardian and lands on-chain' },
    caption: 'One tap approves 35 USDC. The program releases it only with a guardian’s signature; owners cannot approve their own.',
    kiosk: [{ at: 0, screen: 'family', familyBalance: '330 USDC' }],
    app: [
      { do: 'click', key: 'tabs.approvals' },
      { do: 'click', key: 'approvals.approve' },
      { do: 'wait', key: 'notice.approved' },
    ],
  },
  {
    id: 'kiosk', seconds: 12,
    link: { via: 'Solana devnet', dir: 'both', label: 'each device reads the chain on its own' },
    caption: 'At the kiosk, the elder’s own wallet has no daily limit: pick a person, pick an amount, confirm.',
    kiosk: [
      { at: 0, screen: 'wallet' },
      { at: 2.5, tap: { x: 0.36, y: 0.39 }, screen: 'amount', name: ANNA },
      { at: 5, tap: { x: 0.5, y: 0.38 }, screen: 'confirm', name: ANNA, amount: '20 USDC' },
      { at: 7.5, tap: { x: 0.25, y: 0.73 }, screen: 'ok', name: ANNA },
      { at: 10.5, tap: { x: 0.5, y: 0.78 }, screen: 'wallet' },
    ],
    app: [],
  },
];
