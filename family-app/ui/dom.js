// Tiny DOM builder. Text is ALWAYS inserted as text nodes, never parsed as
// HTML: names and addresses come from the chain and are untrusted.
const ALLOWED = new Set([
  'class', 'id', 'type', 'placeholder', 'inputmode', 'autocomplete', 'spellcheck',
  'maxlength', 'value', 'selected', 'checked', 'disabled', 'name', 'for', 'href',
  'role', 'title', 'lang', 'hidden',
]);
const SAFE_HREF = /^(#|\/([^/\\\t\r\n]|$)|https?:\/\/)/;
const CONTROL = /[\t\r\n]/;

export function h(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [key, value] of Object.entries(props || {})) {
    if (value == null || value === false) continue;
    if (key.startsWith('on')) {
      if (typeof value !== 'function') throw new TypeError(`attribute ${key} needs a function`);
      el.addEventListener(key.slice(2).toLowerCase(), value);
      continue;
    }
    if (!ALLOWED.has(key) && !key.startsWith('aria-') && !key.startsWith('data-')) {
      throw new TypeError(`attribute not allowed: ${key}`);
    }
    if (key === 'href' && (CONTROL.test(String(value)) || !SAFE_HREF.test(String(value)))) {
      throw new TypeError(`attribute href has an unsafe value`);
    }
    if (key === 'class') el.className = value;
    else if (value === true) el.setAttribute(key, '');
    else el.setAttribute(key, String(value));
  }
  for (const child of children.flat(Infinity)) {
    if (child == null || child === false) continue;
    el.append(child instanceof Node ? child : document.createTextNode(String(child)));
  }
  return el;
}

export function clear(el) {
  el.replaceChildren();
  return el;
}
