/**
 * AUTO-GENERATED from target/idl/family_vault.json — do not edit by hand.
 * Regenerate after any change to programs/family-vault.
 *
 * `FamilyVault` is the camelCased IDL type (same as Anchor's target/types output);
 * `IDL` is the raw IDL JSON that the Anchor client consumes at runtime.
 */
import idlJson from './family_vault.json';

export type FamilyVault = {
  "address": "38eQgWLzqqaaBPWzbcjYga6pp46GuGn5u2BhTwnJaLz9",
  "metadata": {
    "name": "familyVault",
    "version": "0.1.0",
    "spec": "0.1.0"
  },
  "instructions": [
    {
      "name": "addBeneficiary",
      "discriminator": [
        105,
        214,
        106,
        141,
        180,
        166,
        123,
        238
      ],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "vault",
          "relations": [
            "beneficiaryList"
          ]
        },
        {
          "name": "beneficiaryList",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "name",
          "type": "string"
        },
        {
          "name": "address",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "addBeneficiaryUrgent",
      "discriminator": [
        165,
        35,
        241,
        172,
        208,
        195,
        139,
        241
      ],
      "accounts": [
        {
          "name": "approverOne",
          "signer": true
        },
        {
          "name": "approverTwo",
          "signer": true
        },
        {
          "name": "vault",
          "relations": [
            "approvers",
            "beneficiaryList"
          ]
        },
        {
          "name": "approvers"
        },
        {
          "name": "beneficiaryList",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "name",
          "type": "string"
        },
        {
          "name": "address",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "addTrustedApprover",
      "discriminator": [
        180,
        152,
        190,
        205,
        170,
        55,
        62,
        44
      ],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "vault",
          "relations": [
            "approvers"
          ]
        },
        {
          "name": "approvers",
          "writable": true
        },
        {
          "name": "existingApprover",
          "docs": [
            "Required (must co-sign) when adding a second-or-later approver to a",
            "personal vault; otherwise may be omitted (`None`)."
          ],
          "signer": true,
          "optional": true
        }
      ],
      "args": [
        {
          "name": "address",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "approveWithdrawal",
      "discriminator": [
        75,
        48,
        146,
        122,
        201,
        158,
        210,
        123
      ],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "pendingWithdrawal",
          "writable": true
        },
        {
          "name": "approvers"
        },
        {
          "name": "vaultTokenAccount",
          "writable": true
        },
        {
          "name": "recipientTokenAccount",
          "writable": true
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "pendingWithdrawal",
            "approvers"
          ]
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": []
    },
    {
      "name": "cancelOwnerRotation",
      "discriminator": [
        39,
        81,
        105,
        231,
        8,
        133,
        175,
        150
      ],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "vault",
          "relations": [
            "approvers",
            "rotation"
          ]
        },
        {
          "name": "approvers"
        },
        {
          "name": "rotation",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "deposit",
      "discriminator": [
        242,
        35,
        198,
        137,
        82,
        225,
        242,
        182
      ],
      "accounts": [
        {
          "name": "depositor",
          "signer": true
        },
        {
          "name": "depositorTokenAccount",
          "writable": true
        },
        {
          "name": "vaultTokenAccount",
          "writable": true
        },
        {
          "name": "vault"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "executeOwnerRotation",
      "discriminator": [
        249,
        170,
        199,
        242,
        181,
        65,
        124,
        90
      ],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "rotation"
          ]
        },
        {
          "name": "rotation",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "initializeApprovers",
      "discriminator": [
        107,
        181,
        254,
        161,
        180,
        128,
        81,
        225
      ],
      "accounts": [
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault"
        },
        {
          "name": "approvers",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  112,
                  112,
                  114,
                  111,
                  118,
                  101,
                  114,
                  115
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "initializeVault",
      "discriminator": [
        48,
        191,
        163,
        44,
        71,
        129,
        63,
        164
      ],
      "accounts": [
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "owner",
          "signer": true
        },
        {
          "name": "vault",
          "writable": true
        },
        {
          "name": "beneficiaryList",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  101,
                  110,
                  101,
                  102,
                  105,
                  99,
                  105,
                  97,
                  114,
                  105,
                  101,
                  115
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              }
            ]
          }
        },
        {
          "name": "mint"
        },
        {
          "name": "vaultTokenAccount",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "vault"
              },
              {
                "kind": "const",
                "value": [
                  6,
                  221,
                  246,
                  225,
                  215,
                  101,
                  161,
                  147,
                  217,
                  203,
                  225,
                  70,
                  206,
                  235,
                  121,
                  172,
                  28,
                  180,
                  133,
                  237,
                  95,
                  91,
                  55,
                  145,
                  58,
                  140,
                  245,
                  133,
                  126,
                  255,
                  0,
                  169
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "isPool",
          "type": "bool"
        },
        {
          "name": "dailyLimit",
          "type": "u64"
        }
      ]
    },
    {
      "name": "requestOwnerRotation",
      "discriminator": [
        198,
        42,
        144,
        40,
        100,
        139,
        245,
        219
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault",
          "relations": [
            "approvers"
          ]
        },
        {
          "name": "approvers"
        },
        {
          "name": "rotation",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  111,
                  116,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              },
              {
                "kind": "arg",
                "path": "nonce"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "newOwner",
          "type": "pubkey"
        },
        {
          "name": "nonce",
          "type": "u64"
        }
      ]
    },
    {
      "name": "requestWithdrawal",
      "discriminator": [
        251,
        85,
        121,
        205,
        56,
        201,
        12,
        177
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "beneficiaryList"
          ]
        },
        {
          "name": "beneficiaryList"
        },
        {
          "name": "recipientTokenAccount"
        },
        {
          "name": "pendingWithdrawal",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  113,
                  117,
                  101,
                  115,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "vault"
              },
              {
                "kind": "arg",
                "path": "nonce"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "nonce",
          "type": "u64"
        }
      ]
    },
    {
      "name": "withdraw",
      "discriminator": [
        183,
        18,
        70,
        156,
        148,
        109,
        161,
        34
      ],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "vaultTokenAccount",
          "writable": true
        },
        {
          "name": "recipientTokenAccount",
          "writable": true
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "beneficiaryList"
          ]
        },
        {
          "name": "beneficiaryList"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "beneficiaryList",
      "discriminator": [
        108,
        99,
        165,
        13,
        225,
        127,
        232,
        206
      ]
    },
    {
      "name": "pendingRotation",
      "discriminator": [
        106,
        14,
        119,
        215,
        174,
        177,
        242,
        97
      ]
    },
    {
      "name": "pendingWithdrawal",
      "discriminator": [
        61,
        103,
        179,
        177,
        148,
        199,
        63,
        171
      ]
    },
    {
      "name": "trustedApprovers",
      "discriminator": [
        97,
        204,
        189,
        162,
        190,
        93,
        28,
        154
      ]
    },
    {
      "name": "vault",
      "discriminator": [
        211,
        8,
        232,
        43,
        2,
        152,
        117,
        119
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "notVaultOwner",
      "msg": "Only the vault owner may perform this action"
    },
    {
      "code": 6001,
      "name": "beneficiaryListFull",
      "msg": "Beneficiary list is full"
    },
    {
      "code": 6002,
      "name": "beneficiaryAlreadyExists",
      "msg": "Beneficiary already exists"
    },
    {
      "code": 6003,
      "name": "nameTooLong",
      "msg": "Beneficiary name exceeds maximum length"
    },
    {
      "code": 6004,
      "name": "unknownBeneficiary",
      "msg": "Recipient is not an approved beneficiary"
    },
    {
      "code": 6005,
      "name": "beneficiaryCooldownActive",
      "msg": "Beneficiary cooldown has not elapsed yet"
    },
    {
      "code": 6006,
      "name": "amountOverflow",
      "msg": "Amount overflowed while adding to spent_today"
    },
    {
      "code": 6007,
      "name": "dailyLimitExceeded",
      "msg": "Withdrawal would exceed the vault's daily limit"
    },
    {
      "code": 6008,
      "name": "invalidDailyLimit",
      "msg": "Pool vault daily_limit must be greater than zero"
    },
    {
      "code": 6009,
      "name": "zeroAmount",
      "msg": "Amount must be greater than zero"
    },
    {
      "code": 6010,
      "name": "approverListFull",
      "msg": "Trusted approver list is full"
    },
    {
      "code": 6011,
      "name": "approverAlreadyExists",
      "msg": "Address is already a trusted approver"
    },
    {
      "code": 6012,
      "name": "notPoolVault",
      "msg": "This instruction is only valid for pool vaults"
    },
    {
      "code": 6013,
      "name": "requestAlreadyExecuted",
      "msg": "This withdrawal request has already been executed"
    },
    {
      "code": 6014,
      "name": "notTrustedApprover",
      "msg": "Signer is not a trusted approver"
    },
    {
      "code": 6015,
      "name": "rotationAlreadyCancelled",
      "msg": "This rotation request has already been cancelled"
    },
    {
      "code": 6016,
      "name": "notAuthorizedToCancel",
      "msg": "Only the vault owner or a trusted approver may cancel a rotation"
    },
    {
      "code": 6017,
      "name": "rotationTimelockNotElapsed",
      "msg": "The rotation timelock has not elapsed yet"
    },
    {
      "code": 6018,
      "name": "existingApproverRequired",
      "msg": "An existing trusted approver must co-sign to add another approver to a personal vault"
    },
    {
      "code": 6019,
      "name": "amountDoesNotExceedDailyLimit",
      "msg": "Requested amount does not exceed the daily limit; use ordinary withdraw instead"
    },
    {
      "code": 6020,
      "name": "cannotApproveOwnWithdrawals",
      "msg": "The vault owner may not be added as a trusted approver"
    },
    {
      "code": 6021,
      "name": "cannotRotateToSelf",
      "msg": "An approver may not name themselves as the new owner"
    },
    {
      "code": 6022,
      "name": "invalidApproverAddress",
      "msg": "Approver address may not be the default (all-zeros) pubkey"
    },
    {
      "code": 6023,
      "name": "duplicateApprover",
      "msg": "The two approvers co-signing an urgent beneficiary add must be different people"
    }
  ],
  "types": [
    {
      "name": "beneficiary",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "address",
            "type": "pubkey"
          },
          {
            "name": "name",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "nameLen",
            "type": "u8"
          },
          {
            "name": "addedAt",
            "type": "i64"
          },
          {
            "name": "activeAfter",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "beneficiaryList",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "count",
            "type": "u8"
          },
          {
            "name": "entries",
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "beneficiary"
                  }
                },
                10
              ]
            }
          }
        ]
      }
    },
    {
      "name": "pendingRotation",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "newOwner",
            "type": "pubkey"
          },
          {
            "name": "requestedBy",
            "type": "pubkey"
          },
          {
            "name": "requestedAt",
            "type": "i64"
          },
          {
            "name": "executesAt",
            "type": "i64"
          },
          {
            "name": "cancelled",
            "type": "bool"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "executed",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "pendingWithdrawal",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "to",
            "type": "pubkey"
          },
          {
            "name": "requestedAt",
            "type": "i64"
          },
          {
            "name": "approvedBy",
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "executed",
            "type": "bool"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "trustedApprovers",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "count",
            "type": "u8"
          },
          {
            "name": "entries",
            "type": {
              "array": [
                "pubkey",
                5
              ]
            }
          }
        ]
      }
    },
    {
      "name": "vault",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "vaultTokenAccount",
            "type": "pubkey"
          },
          {
            "name": "dailyLimit",
            "type": "u64"
          },
          {
            "name": "spentToday",
            "type": "u64"
          },
          {
            "name": "lastResetDay",
            "type": "i64"
          },
          {
            "name": "isPool",
            "type": "bool"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "seedOwner",
            "type": "pubkey"
          }
        ]
      }
    }
  ]
};

export const IDL = idlJson as unknown as FamilyVault;
