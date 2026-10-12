export const APP_URL = '?mock=demo&lang=en';
export const KIOSK_SCREENS = ['wallet', 'family', 'amount', 'confirm', 'ok', 'fail', 'setup', 'wait'];

const ANNA = 'Granddaughter Anna';
const PERSONAL = 'Cw5RnE8yMq2KdLtV7XbZa3JhUp9GfoSi1TmBvNeY4AkD'; // the elder’s account in the mock

export const scenes = [
  {
    id: 'ready', seconds: 7,
    caption: 'Trellis: the elder uses a simple kiosk. A relative opens the family app.',
    kiosk: { screen: 'wallet' },
    app: [{ do: 'wait', key: 'pair.title' }],
  },
  {
    id: 'pairing', seconds: 9,
    caption: 'Pairing (simulated): the kiosk shows a one-time code, the relative types it.',
    kiosk: { screen: 'setup' },
    app: [
      { do: 'fill', selector: 'input[autocomplete=name]', value: 'Sasha' },
      { do: 'fill', selector: 'input[placeholder^="fvpair1"]', value: '123456' },
    ],
  },
  {
    id: 'waiting', seconds: 7,
    caption: 'Handed over to the device (simulated). The app waits to be added.',
    kiosk: { screen: 'setup' },
    app: [{ do: 'click', key: 'pair.submit' }, { do: 'wait', key: 'waiting.title' }],
  },
  {
    id: 'family', seconds: 9,
    caption: 'Family budget: shared money with a daily limit of 30 USDC, enforced by the Solana program.',
    kiosk: { screen: 'family' },
    app: [{ do: 'wait', key: 'home.title_pool' }],
  },
  {
    id: 'within-limit', seconds: 9,
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
    caption: 'A newly added person waits 48 hours before they can be paid, so a stolen key cannot cash out.',
    kiosk: { screen: 'family' },
    app: [{ do: 'click', key: 'tabs.people' }, { do: 'wait', key: 'people.payees' }],
  },
  {
    id: 'personal', seconds: 9,
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
    caption: 'A transfer of 35 USDC waits for the guardian. One tap approves it (simulated kiosk request).',
    kiosk: { screen: 'confirm', name: ANNA, amount: '35 USDC' },
    app: [
      { do: 'click', key: 'tabs.approvals' },
      { do: 'click', key: 'approvals.approve' },
      { do: 'wait', key: 'notice.approved' },
    ],
  },
  {
    id: 'done', seconds: 8,
    caption: 'Done. Plain words and big buttons for the elder, control for the family.',
    kiosk: { screen: 'ok', name: ANNA },
    app: [],
  },
];
