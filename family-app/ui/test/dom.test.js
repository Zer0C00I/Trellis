import test from 'node:test';
import assert from 'node:assert/strict';

class FakeNode {}
class FakeText extends FakeNode {
  constructor(data) { super(); this.nodeType = 3; this.data = data; }
}
class FakeEl extends FakeNode {
  constructor(tag) {
    super();
    this.tag = tag; this.className = ''; this.attrs = {}; this.listeners = []; this.children = [];
  }
  setAttribute(k, v) { this.attrs[k] = v; }
  getAttribute(k) { return this.attrs[k]; }
  addEventListener(name, fn) { this.listeners.push([name, fn]); }
  append(...kids) { this.children.push(...kids); }
  replaceChildren() { this.children = []; }
}
globalThis.Node = FakeNode;
globalThis.document = {
  createElement: (t) => new FakeEl(t),
  createTextNode: (d) => new FakeText(d),
};
const { h, clear } = await import('../dom.js');

test('hostile string child stays a text node', () => {
  const el = h('div', {}, '<img src=x onerror=alert(1)>');
  assert.equal(el.children.length, 1);
  assert.equal(el.children[0].nodeType, 3);
  assert.equal(el.children[0].data, '<img src=x onerror=alert(1)>');
});

test('forbidden props throw TypeError naming the attribute', () => {
  for (const key of ['innerHTML', 'outerHTML', 'srcdoc', 'style', 'onclick-x', 'src', 'formaction']) {
    assert.throws(() => h('div', { [key]: 'x' }), (e) => e instanceof TypeError && e.message.includes(key));
  }
  assert.throws(() => h('div', { onclick: 'alert(1)' }), (e) => e instanceof TypeError && e.message.includes('onclick'));
  assert.throws(() => h('div', { onClick: 'alert(1)' }), TypeError);
});

test('allowlisted props are set', () => {
  const el = h('input', {
    class: 'a b', id: 'i', type: 'text', placeholder: 'p', inputmode: 'numeric', maxlength: 6,
    disabled: true, hidden: false, title: null, 'aria-label': 'L', 'data-x': 1, role: 'button', for: 'f', name: 'n', lang: 'ru',
  });
  assert.equal(el.className, 'a b');
  assert.equal(el.attrs.id, 'i');
  assert.equal(el.attrs.maxlength, '6');
  assert.equal(el.attrs.disabled, '');
  assert.equal('hidden' in el.attrs, false);
  assert.equal('title' in el.attrs, false);
  assert.equal(el.attrs['aria-label'], 'L');
  assert.equal(el.attrs['data-x'], '1');
});

test('function handlers register lowercased event names', () => {
  const f = () => {};
  const g = () => {};
  const el = h('button', { onClick: f, oninput: g });
  assert.deepEqual(el.listeners, [['click', f], ['input', g]]);
});

test('href: only #, /, http(s) allowed', () => {
  assert.equal(h('a', { href: '#/home' }).attrs.href, '#/home');
  assert.equal(h('a', { href: 'https://x' }).attrs.href, 'https://x');
  assert.equal(h('a', { href: 'http://x' }).attrs.href, 'http://x');
  assert.equal(h('a', { href: '/p' }).attrs.href, '/p');
  assert.throws(() => h('a', { href: 'javascript:alert(1)' }), TypeError);
  assert.throws(() => h('a', { href: 'data:text/html,x' }), TypeError);
});

test('children: nested arrays flatten, 0 kept, null/false skipped', () => {
  const inner = h('b');
  const el = h('div', null, ['a', [['b', [inner]]], 0, null, false, undefined]);
  assert.deepEqual(el.children.map((c) => c.data ?? c), ['a', 'b', inner, '0']);
});

test('clear empties the element', () => {
  const el = h('div', {}, 'x', 'y');
  assert.equal(clear(el), el);
  assert.equal(el.children.length, 0);
});
