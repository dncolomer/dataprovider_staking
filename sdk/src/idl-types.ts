/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/dataprovider_staking.json`.
 */
export type DataproviderStaking = {
  "address": "94Ja6Y8AuzmZHjQiyk2SzvoysnBr3F17nfHGrHm1idAZ",
  "metadata": {
    "name": "dataproviderStaking",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "GHC1CHEM multi-mint staking with USDC dividend distribution"
  },
  "instructions": [
    {
      "name": "acceptAdmin",
      "docs": [
        "New admin: accept a proposed admin rotation."
      ],
      "discriminator": [
        112,
        42,
        45,
        90,
        116,
        181,
        13,
        170
      ],
      "accounts": [
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "newAdmin",
          "docs": [
            "The new admin; must equal `config.pending_admin` and sign."
          ],
          "signer": true
        }
      ],
      "args": []
    },
    {
      "name": "addPool",
      "docs": [
        "Admin: register a new stake-mint pool (with stake + reward vaults)."
      ],
      "discriminator": [
        115,
        230,
        212,
        211,
        175,
        49,
        39,
        169
      ],
      "accounts": [
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "stakeMint"
        },
        {
          "name": "usdcMint",
          "docs": [
            "USDC mint; must match the one recorded at `initialize`."
          ]
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              }
            ]
          }
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "PDA authority that will own both vaults. Not deserialized; only used",
            "to derive its bump."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              }
            ]
          }
        },
        {
          "name": "stakeVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  116,
                  97,
                  107,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              }
            ]
          }
        },
        {
          "name": "rewardVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  119,
                  97,
                  114,
                  100,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              }
            ]
          }
        },
        {
          "name": "stakeTokenProgram",
          "docs": [
            "Token program matching the `stake_mint` owner (SPL Token or Token-2022)."
          ]
        },
        {
          "name": "usdcTokenProgram",
          "docs": [
            "Token program matching the `usdc_mint` owner (SPL Token or Token-2022).",
            "Usually classic SPL Token for mainnet USDC."
          ]
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "cancelAdminProposal",
      "docs": [
        "Admin: cancel a pending admin rotation."
      ],
      "discriminator": [
        68,
        6,
        145,
        131,
        16,
        73,
        182,
        229
      ],
      "accounts": [
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "config"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "claimRewards",
      "docs": [
        "User: claim all pending USDC rewards from a pool."
      ],
      "discriminator": [
        4,
        144,
        132,
        71,
        116,
        23,
        151,
        80
      ],
      "accounts": [
        {
          "name": "user",
          "signer": true
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "stakeMint",
          "relations": [
            "pool"
          ]
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              }
            ]
          }
        },
        {
          "name": "rewardVault",
          "writable": true,
          "relations": [
            "pool"
          ]
        },
        {
          "name": "usdcMint"
        },
        {
          "name": "vaultAuthority",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              }
            ]
          }
        },
        {
          "name": "userStake",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  117,
                  115,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              },
              {
                "kind": "account",
                "path": "user"
              }
            ]
          }
        },
        {
          "name": "userUsdcAccount",
          "writable": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program matching the USDC mint (classic SPL on mainnet today)."
          ]
        }
      ],
      "args": []
    },
    {
      "name": "depositRewards",
      "docs": [
        "Admin: deposit `amount` USDC into a pool's reward vault. Distributed",
        "pro-rata to current stakers via the reward-per-share accumulator."
      ],
      "discriminator": [
        52,
        249,
        112,
        72,
        206,
        161,
        196,
        1
      ],
      "accounts": [
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "stakeMint",
          "relations": [
            "pool"
          ]
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              }
            ]
          }
        },
        {
          "name": "rewardVault",
          "writable": true,
          "relations": [
            "pool"
          ]
        },
        {
          "name": "usdcMint"
        },
        {
          "name": "adminUsdcAccount",
          "docs": [
            "The admin's USDC source account."
          ],
          "writable": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program matching the USDC mint (classic SPL on mainnet today)."
          ]
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
      "name": "initialize",
      "docs": [
        "Bootstraps the program. Signer becomes the initial admin."
      ],
      "discriminator": [
        175,
        175,
        109,
        31,
        13,
        152,
        155,
        237
      ],
      "accounts": [
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "admin",
          "docs": [
            "The initial admin. Must sign so that the admin key is authenticated at",
            "bootstrap (prevents front-running of config creation with a wrong admin)."
          ],
          "signer": true
        },
        {
          "name": "usdcMint",
          "docs": [
            "The USDC mint used as the reward currency for every pool."
          ]
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
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
      "name": "proposeAdmin",
      "docs": [
        "Admin: propose a new admin (2-step transfer)."
      ],
      "discriminator": [
        121,
        214,
        199,
        212,
        87,
        39,
        117,
        234
      ],
      "accounts": [
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "newAdmin",
          "docs": [
            "authentication happens when this pubkey signs `accept_admin`."
          ]
        }
      ],
      "args": []
    },
    {
      "name": "stake",
      "docs": [
        "User: deposit `amount` of `stake_mint` into the pool."
      ],
      "discriminator": [
        206,
        176,
        202,
        18,
        200,
        209,
        179,
        108
      ],
      "accounts": [
        {
          "name": "user",
          "writable": true,
          "signer": true
        },
        {
          "name": "stakeMint",
          "relations": [
            "pool"
          ]
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              }
            ]
          }
        },
        {
          "name": "stakeVault",
          "writable": true,
          "relations": [
            "pool"
          ]
        },
        {
          "name": "userStake",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  117,
                  115,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              },
              {
                "kind": "account",
                "path": "user"
              }
            ]
          }
        },
        {
          "name": "userTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
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
      "name": "unstake",
      "docs": [
        "User: withdraw `amount` of their staked tokens from the pool."
      ],
      "discriminator": [
        90,
        95,
        107,
        42,
        205,
        124,
        50,
        225
      ],
      "accounts": [
        {
          "name": "user",
          "signer": true
        },
        {
          "name": "stakeMint",
          "relations": [
            "pool"
          ]
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              }
            ]
          }
        },
        {
          "name": "stakeVault",
          "writable": true,
          "relations": [
            "pool"
          ]
        },
        {
          "name": "vaultAuthority",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              }
            ]
          }
        },
        {
          "name": "userStake",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  117,
                  115,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "stakeMint"
              },
              {
                "kind": "account",
                "path": "user"
              }
            ]
          }
        },
        {
          "name": "userTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram"
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
      "name": "globalConfig",
      "discriminator": [
        149,
        8,
        156,
        202,
        160,
        252,
        176,
        217
      ]
    },
    {
      "name": "tokenPool",
      "discriminator": [
        103,
        51,
        150,
        210,
        226,
        131,
        104,
        33
      ]
    },
    {
      "name": "userStake",
      "discriminator": [
        102,
        53,
        163,
        107,
        9,
        138,
        87,
        153
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "mathOverflow",
      "msg": "Arithmetic overflow."
    },
    {
      "code": 6001,
      "name": "unauthorized",
      "msg": "Caller is not the program admin."
    },
    {
      "code": 6002,
      "name": "notPendingAdmin",
      "msg": "Caller is not the pending admin for this handover."
    },
    {
      "code": 6003,
      "name": "noPendingAdmin",
      "msg": "No pending admin handover in progress."
    },
    {
      "code": 6004,
      "name": "maxPoolsReached",
      "msg": "Maximum number of pools already created."
    },
    {
      "code": 6005,
      "name": "poolAlreadyExists",
      "msg": "Pool already exists for this mint."
    },
    {
      "code": 6006,
      "name": "zeroAmount",
      "msg": "Amount must be greater than zero."
    },
    {
      "code": 6007,
      "name": "insufficientStake",
      "msg": "Insufficient staked balance for requested withdrawal."
    },
    {
      "code": 6008,
      "name": "nothingToClaim",
      "msg": "No rewards available to claim."
    },
    {
      "code": 6009,
      "name": "noStakersInPool",
      "msg": "Pool has no stakers yet; rewards cannot be distributed."
    },
    {
      "code": 6010,
      "name": "invalidRewardMint",
      "msg": "Reward vault mint does not match the USDC mint configured in GlobalConfig."
    },
    {
      "code": 6011,
      "name": "invalidStakeMint",
      "msg": "Stake vault mint does not match the pool's stake mint."
    },
    {
      "code": 6012,
      "name": "rewardDepositTooSmall",
      "msg": "Reward deposit too small to distribute; would round to zero."
    }
  ],
  "types": [
    {
      "name": "globalConfig",
      "docs": [
        "Singleton config account. Holds the admin authority (updateable only by the",
        "current admin) and the USDC mint used for every pool's reward vault."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "docs": [
              "Authority permitted to add pools, deposit rewards, and rotate the admin."
            ],
            "type": "pubkey"
          },
          {
            "name": "pendingAdmin",
            "docs": [
              "Pending admin for 2-step admin rotation. `Pubkey::default()` when no",
              "handover is in progress."
            ],
            "type": "pubkey"
          },
          {
            "name": "usdcMint",
            "docs": [
              "USDC mint used as the reward currency for every pool."
            ],
            "type": "pubkey"
          },
          {
            "name": "poolCount",
            "docs": [
              "Number of pools that have been created so far."
            ],
            "type": "u8"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump."
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "tokenPool",
      "docs": [
        "Per-stake-mint pool. Holds the staking vault, reward (USDC) vault, and the",
        "running pro-rata accumulator used to compute each user's earnings."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "stakeMint",
            "docs": [
              "The SPL mint users stake into this pool."
            ],
            "type": "pubkey"
          },
          {
            "name": "stakeVault",
            "docs": [
              "PDA-owned token account holding all staked tokens for this pool."
            ],
            "type": "pubkey"
          },
          {
            "name": "rewardVault",
            "docs": [
              "PDA-owned token account holding all undistributed USDC rewards."
            ],
            "type": "pubkey"
          },
          {
            "name": "totalStaked",
            "docs": [
              "Total tokens currently staked across all users in this pool."
            ],
            "type": "u64"
          },
          {
            "name": "accRewardPerShare",
            "docs": [
              "Accumulated USDC reward per staked token unit, scaled by `ACC_PRECISION`.",
              "",
              "Invariant: increases monotonically every time rewards are deposited",
              "while `total_staked > 0`."
            ],
            "type": "u128"
          },
          {
            "name": "totalRewardsDeposited",
            "docs": [
              "Lifetime USDC deposited as rewards into this pool (for accounting)."
            ],
            "type": "u64"
          },
          {
            "name": "totalRewardsClaimed",
            "docs": [
              "Lifetime USDC actually claimed by users from this pool."
            ],
            "type": "u64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump for the pool account."
            ],
            "type": "u8"
          },
          {
            "name": "vaultAuthorityBump",
            "docs": [
              "PDA bump for the stake vault authority."
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "userStake",
      "docs": [
        "Per-user, per-pool staking position."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "docs": [
              "Owner of this stake position."
            ],
            "type": "pubkey"
          },
          {
            "name": "stakeMint",
            "docs": [
              "Pool this position belongs to (redundant but useful for indexers)."
            ],
            "type": "pubkey"
          },
          {
            "name": "amount",
            "docs": [
              "Tokens currently staked by the owner in this pool."
            ],
            "type": "u64"
          },
          {
            "name": "rewardDebt",
            "docs": [
              "Reward-debt in MasterChef accounting. Equals `amount * acc_reward_per_share / ACC_PRECISION`",
              "at the time of the last settlement."
            ],
            "type": "u128"
          },
          {
            "name": "pendingRewards",
            "docs": [
              "Rewards already settled to the user's \"claimable\" bucket but not yet",
              "transferred out of the reward vault."
            ],
            "type": "u64"
          },
          {
            "name": "totalClaimed",
            "docs": [
              "Lifetime USDC claimed by this user from this pool."
            ],
            "type": "u64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump."
            ],
            "type": "u8"
          }
        ]
      }
    }
  ],
  "constants": [
    {
      "name": "configSeed",
      "docs": [
        "PDA seed for `GlobalConfig`."
      ],
      "type": "bytes",
      "value": "[99, 111, 110, 102, 105, 103]"
    },
    {
      "name": "poolSeed",
      "docs": [
        "PDA seed prefix for a `TokenPool`. Full seeds: [`POOL_SEED`, stake_mint]."
      ],
      "type": "bytes",
      "value": "[112, 111, 111, 108]"
    },
    {
      "name": "userSeed",
      "docs": [
        "PDA seed prefix for a `UserStake`. Full seeds: [`USER_SEED`, stake_mint, owner]."
      ],
      "type": "bytes",
      "value": "[117, 115, 101, 114]"
    },
    {
      "name": "vaultAuthSeed",
      "docs": [
        "PDA seed prefix for a pool's stake-vault authority. Full seeds:",
        "[`VAULT_AUTH_SEED`, stake_mint]. This PDA owns both the stake-vault",
        "and the reward-vault token accounts for the pool."
      ],
      "type": "bytes",
      "value": "[118, 97, 117, 108, 116, 95, 97, 117, 116, 104]"
    }
  ]
};
