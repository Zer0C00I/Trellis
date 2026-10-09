// What the person typed or pasted on the first-launch screen: the text of the
// QR code on the device ("fvpair1:..."), or the six-digit backup code.
export function classifyPairingInput(text) {
  const value = String(text ?? '').trim();
  if (value === '') return { kind: 'empty' };
  if (/^\d{6}$/.test(value)) return { kind: 'pin', value };
  if (value.startsWith('fvpair1:')) return { kind: 'qr', value };
  return { kind: 'unknown' };
}
