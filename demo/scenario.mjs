export const APP_URL = '?mock=demo&lang=en';
export const KIOSK_SCREENS = ['wallet', 'family', 'amount', 'confirm', 'ok', 'fail', 'setup', 'wait'];

const ANNA = 'Granddaughter Anna';
const PERSONAL = 'Cw5RnE8yMq2KdLtV7XbZa3JhUp9GfoSi1TmBvNeY4AkD'; // the elder’s account in the mock

// link.dir is spatial: 'right' = from the kiosk (left) to the app (right), 'left' = from the app to the kiosk.
export const scenes = [
  {
    id: 'ready', seconds: 7,
    link: null,
    caption: 'Trellis: the elder uses a simple kiosk. A relative opens the family app.',
    kiosk: { screen: 'wallet' },
    app: [{ do: 'wait', key: 'pair.title' }],
  },
  {
    id: 'pairing', seconds: 9,
    link: { via: 'Bluetooth LE', dir: 'left', label: '6-digit code 123456 goes from the phone to the kiosk' },
    caption: 'Pairing: the kiosk shows a one-time code, the relative types it.',
    kiosk: { screen: 'setup' },
    app: [
      { do: 'fill', selector: 'input[autocomplete=name]', value: 'Sasha' },
      { do: 'fill', selector: 'input[placeholder^="fvpair1"]', value: '123456' },
    ],
  },
  {
    id: 'waiting', seconds: 7,
    link: { via: 'Solana devnet', dir: 'right', label: 'kiosk registers the relative as a guardian on-chain' },
    caption: 'Handed over to the device. The app waits to be added.',
    kiosk: { screen: 'setup' },
    app: [{ do: 'click', key: 'pair.submit' }, { do: 'wait', key: 'waiting.title' }],
  },
  {
    id: 'family', seconds: 9,
    link: { via: 'Solana devnet', dir: 'both', label: 'both screens read the same vault state' },
    caption: 'Family budget: shared money with a daily limit of 30 USDC, enforced by the Solana program.',
    kiosk: { screen: 'family' },
    app: [{ do: 'wait', key: 'home.title_pool' }],
  },
  {
    id: 'within-limit', seconds: 9,
    link: { via: 'Solana devnet', dir: 'left', label: 'send_money 12 USDC, confirmed on-chain' },
    caption: 'Within the limit, 12 USDC for a granddaughter goes through at once.',
    kiosk: { screen: 'ok', name: ANNA },
    app: [
      { do: 'click', key: 'tabs.send' },
      { do: 'fill', selector: 'input[inputmode=decimal]', value: '12' },
      { do: 'click', key: 'send.submit' },
      { do: 'wait', key: 'notice.sent' },
    ],
  },
  {
    id: 'over-limit', seconds: 9,
    link: { via: 'Solana devnet', dir: 'left', label: '45 USDC is above the daily limit: filed as a request, not paid' },
    caption: 'Above the limit, 45 USDC is not paid. It becomes a request that a guardian must approve.',
    kiosk: { screen: 'family' },
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
    kiosk: { screen: 'family' },
    app: [{ do: 'click', key: 'tabs.people' }, { do: 'wait', key: 'people.payees' }],
  },
  {
    id: 'personal', seconds: 9,
    link: { via: 'Solana devnet', dir: 'right', label: 'load_vault: the elder’s account, the relative is a guardian' },
    caption: 'Personal account: the elder’s own money. The relative is only a guardian.',
    kiosk: { screen: 'wallet' },
    app: [
      { do: 'select', selector: 'select[aria-label]', value: PERSONAL },
      { do: 'click', key: 'tabs.home' },
      { do: 'wait', key: 'home.title_personal' },
    ],
  },
  {
    id: 'approve', seconds: 10,
    link: { via: 'Solana devnet', dir: 'left', label: 'approve_request signed by the guardian releases the 35 USDC' },
    caption: 'A transfer of 35 USDC waits for the guardian. One tap approves it.',
    kiosk: { screen: 'confirm', name: ANNA, amount: '35 USDC' },
    app: [
      { do: 'click', key: 'tabs.approvals' },
      { do: 'click', key: 'approvals.approve' },
      { do: 'wait', key: 'notice.approved' },
    ],
  },
  {
    id: 'done', seconds: 8,
    link: { via: 'Solana devnet', dir: 'both', label: 'transaction confirmed, both screens show the result' },
    caption: 'Done. Plain words and big buttons for the elder, control for the family.',
    kiosk: { screen: 'ok', name: ANNA },
    app: [],
  },
];
