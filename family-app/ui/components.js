import { h } from './dom.js';

export function banner(kind, text, ...extra) {
  return h('div', { class: `banner banner-${kind}`, role: kind === 'error' ? 'alert' : 'status' }, h('p', {}, text), ...extra);
}

// An ErrorDto from Rust: the plain message, details tucked away.
export function errorBanner(e) {
  if (!e) return null;
  return banner(
    e.kind === 'unconfirmed' ? 'warn' : 'error',
    e.message,
    e.details ? h('details', {}, h('summary', {}, 'Подробности'), h('code', {}, e.details)) : null,
  );
}

export function button(label, onclick, opts = {}) {
  return h('button', { type: 'button', class: `btn ${opts.secondary ? 'btn-secondary' : ''}`, onclick, disabled: opts.disabled }, label);
}

export function field(label, input, hint) {
  const id = `f-${Math.random().toString(36).slice(2, 8)}`;
  input.id = id;
  return h('div', { class: 'field' }, h('label', { for: id }, label), input, hint ? h('p', { class: 'hint' }, hint) : null);
}

export function card(title, ...children) {
  return h('section', { class: 'card' }, title ? h('h2', {}, title) : null, ...children);
}

// Runs `work` with the button disabled. Returns { ok: true, value } or, after
// showing the error in `out`, { ok: false }. An "unconfirmed" error also tells
// the shell to re-read the pending operation so the warning banner appears.
export async function withFeedback(btn, out, work) {
  btn.disabled = true;
  out.replaceChildren();
  try {
    return { ok: true, value: await work() };
  } catch (e) {
    out.append(errorBanner(e));
    if (e.kind === 'unconfirmed') document.dispatchEvent(new CustomEvent('trellis:pending'));
    return { ok: false, value: null };
  } finally {
    btn.disabled = false;
  }
}
