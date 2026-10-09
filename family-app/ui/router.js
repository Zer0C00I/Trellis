// Hash routes: "#/home", "#/send", ... Unknown or empty -> "home".
export const ROUTES = ['home', 'send', 'people', 'approvals', 'me'];

export function parseRoute(hash) {
  const name = String(hash ?? '').replace(/^#\/?/, '').split('?')[0];
  return ROUTES.includes(name) ? name : 'home';
}
