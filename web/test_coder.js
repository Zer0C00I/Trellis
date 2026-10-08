const anchor = require('@coral-xyz/anchor');
const IDL = {
  "version": "0.1.0",
  "name": "family_vault",
  "instructions": [
    {
      "name": "initializeVault",
      "accounts": [],
      "args": [
        { "name": "isPool", "type": "bool" },
        { "name": "dailyLimit", "type": "u64" }
      ]
    }
  ]
};
const coder = new anchor.BorshInstructionCoder(IDL);
console.log('coder init success');
try {
  coder.encode('initializeVault', { isPool: false, dailyLimit: new anchor.BN(200) });
  console.log('encode success');
} catch(e) {
  console.log('error:', e.stack);
}
