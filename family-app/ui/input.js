// What the person typed or pasted on the first-launch screen: the text of the
// QR code on the device ("fvpair1:..."), or the six-digit backup code.
export function classifyPairingInput(text) {
  const value = String(text ?? '').trim();
  if (value === '') return { kind: 'empty' };
  if (value.length > 64) return { kind: 'unknown' };
  const digits = value.replace(/[ \t\r\n]+/g, '');
  if (/^\d{6}$/.test(digits)) return { kind: 'pin', value: digits };
  if (/^fvpair1:/i.test(value) && value.length > 'fvpair1:'.length) {
    return { kind: 'qr', value: 'fvpair1:' + value.slice('fvpair1:'.length) };
  }
  return { kind: 'unknown' };
}
