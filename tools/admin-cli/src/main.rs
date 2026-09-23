//! Admin CLI for the `family-vault` program: initialize vaults, manage
//! beneficiaries/approvers, and drive owner-side flows that the kiosk GUI
//! deliberately never exposes to the vault owner's own device (see
//! `docs/superpowers/specs/2026-09-21-kiosk-blockchain-wiring-design.md`,
//! "Вне скоупа этого plan'а" — those operations belong to a family-facing
//! admin surface, which this tool is a command-line stand-in for until a
//! real web app exists).
//!
//! Every subcommand builds one instruction via `family-vault`'s
//! Anchor-generated `instruction`/`accounts` client bindings (the same
//! `anchor_lang::prelude::Pubkey` <-> `solana_pubkey::Pubkey` conversion
//! pattern as `kiosk-gui/src/chain.rs`, since these are two distinct
//! `Pubkey` types from different generations of the Solana crate
//! ecosystem), signs it, and sends it with `send_and_confirm_transaction`.

mod registry;

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_client::rpc_client::RpcClient;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::{read_keypair_file, Keypair};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_token::solana_program::program_pack::Pack;

use registry::{PersonEntry, VaultEntry};
use family_vault_client::{to_anchor, to_client};

/// `family-vault` only exports its declared id as `anchor_lang::prelude::Pubkey`
/// (via `declare_id!`) — this tool needs it in the client-era `solana_pubkey`
/// type for PDA derivation, RPC calls, and `Instruction::program_id`.
fn program_id() -> Pubkey {
    to_client(family_vault::ID)
}

fn vault_pda(owner: Pubkey, is_pool: bool) -> (Pubkey, u8) {
    family_vault_client::vault_pda(&program_id(), &owner, is_pool)
}

fn beneficiary_list_pda(vault: Pubkey) -> (Pubkey, u8) {
    family_vault_client::beneficiary_list_pda(&program_id(), &vault)
}

fn approvers_pda(vault: Pubkey) -> (Pubkey, u8) {
    family_vault_client::approvers_pda(&program_id(), &vault)
}

fn request_pda(vault: Pubkey, nonce: u64) -> (Pubkey, u8) {
    family_vault_client::request_pda(&program_id(), &vault, nonce)
}

fn rotation_pda(vault: Pubkey, nonce: u64) -> (Pubkey, u8) {
    family_vault_client::rotation_pda(&program_id(), &vault, nonce)
}

fn rpc_client(rpc_url: &str) -> RpcClient {
    RpcClient::new(rpc_url.to_string())
}

fn send_ix(
    rpc: &RpcClient,
    accounts: Vec<AccountMeta>,
    data: Vec<u8>,
    payer: &Keypair,
    extra_signers: &[&Keypair],
) -> Result<solana_signature::Signature, String> {
    let ix = Instruction {
        program_id: program_id(),
        accounts,
        data,
    };
    let blockhash = rpc
        .get_latest_blockhash()
        .map_err(|e| format!("get_latest_blockhash: {e}"))?;

    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);

    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &signers, blockhash);
    rpc.send_and_confirm_transaction(&tx)
        .map_err(|e| format!("send_and_confirm_transaction: {e}"))
}

fn arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn arg_req(args: &[String], flag: &str) -> String {
    arg(args, flag).unwrap_or_else(|| {
        eprintln!("missing required flag: {flag}");
        std::process::exit(1);
    })
}

fn arg_pubkey(args: &[String], flag: &str) -> Pubkey {
    arg_req(args, flag).parse().unwrap_or_else(|_| {
        eprintln!("{flag}: not a valid pubkey");
        std::process::exit(1);
    })
}

fn arg_u64(args: &[String], flag: &str) -> u64 {
    arg_req(args, flag).parse().unwrap_or_else(|_| {
        eprintln!("{flag}: not a valid u64");
        std::process::exit(1);
    })
}

fn arg_keypair(args: &[String], flag: &str) -> Keypair {
    let path = arg_req(args, flag);
    read_keypair_file(&path).unwrap_or_else(|e| {
        eprintln!("{flag} ({path}): could not read keypair: {e}");
        std::process::exit(1);
    })
}

fn rpc_url(args: &[String]) -> String {
    arg(args, "--rpc-url").unwrap_or_else(|| "https://api.devnet.solana.com".to_string())
}

fn registry_path(args: &[String]) -> std::path::PathBuf {
    arg(args, "--registry")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(registry::default_path)
}

fn system_program_id() -> Pubkey {
    Pubkey::new_from_array([0u8; 32])
}

/// `send`/`deposit` accept whole-unit amounts (e.g. `--amount 10`) and
/// convert to base units using this, so nobody using this tool has to do
/// that math themselves.
fn fetch_mint_decimals(rpc: &RpcClient, mint: &Pubkey) -> u32 {
    family_vault_client::fetch_mint_decimals(rpc, mint).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    })
}

fn cmd_register_person(args: &[String]) {
    let path = registry_path(args);
    let mut reg = registry::load(&path);
    let name = arg_req(args, "--name");
    let keypair_path = arg(args, "--keypair");
    let explicit_pubkey = arg(args, "--pubkey");

    let (pubkey, keypair_path) = match (&keypair_path, &explicit_pubkey) {
        (Some(kp), None) => {
            let keypair = read_keypair_file(kp).unwrap_or_else(|e| {
                eprintln!("--keypair ({kp}): could not read: {e}");
                std::process::exit(1);
            });
            (keypair.pubkey().to_string(), Some(kp.clone()))
        }
        (None, Some(pk)) => {
            pk.parse::<Pubkey>().unwrap_or_else(|_| {
                eprintln!("--pubkey: not a valid pubkey");
                std::process::exit(1);
            });
            (pk.clone(), None)
        }
        _ => {
            eprintln!(
                "register-person needs exactly one of --keypair <path> or --pubkey <address>"
            );
            std::process::exit(1);
        }
    };

    reg.people.insert(
        name.clone(),
        PersonEntry {
            keypair: keypair_path,
            pubkey: pubkey.clone(),
        },
    );
    registry::save(&path, &reg);
    println!("registered person '{name}' -> {pubkey}");
}

fn cmd_register_vault(args: &[String]) {
    let path = registry_path(args);
    let mut reg = registry::load(&path);
    let name = arg_req(args, "--name");
    let pubkey = arg_pubkey(args, "--pubkey").to_string();
    let mint = arg_pubkey(args, "--mint").to_string();
    let owner = arg_req(args, "--owner");
    reg.people.get(&owner).unwrap_or_else(|| {
        eprintln!("--owner '{owner}': no such person — register them first");
        std::process::exit(1);
    });

    reg.vaults.insert(
        name.clone(),
        VaultEntry {
            pubkey: pubkey.clone(),
            mint,
            owner,
        },
    );
    registry::save(&path, &reg);
    println!("registered vault '{name}' -> {pubkey}");
}

/// Splits a `--approvers a,b,c` / `--beneficiaries a,b,c` flag into
/// registry names, ignoring a missing flag or empty entries (so trailing
/// commas or an unset flag don't produce phantom names).
fn arg_name_list(args: &[String], flag: &str) -> Vec<String> {
    arg(args, flag)
        .map(|s| {
            s.split(',')
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Bank-style: "open an account for <owner>". Runs `initialize_vault`
/// on-chain, then registers the resulting vault under `--name` so later
/// commands (`send`, `deposit`, `add-beneficiary`) never need the raw
/// vault pubkey again.
///
/// For `--pool` vaults, also runs the rest of the family-vault opening
/// flow in one command: `initialize_approvers`, then one
/// `add_trusted_approver` per `--approvers` name and one `add_beneficiary`
/// per `--beneficiaries` name — so a family member never has to chain six
/// separate raw-pubkey commands by hand. Each step is its own transaction,
/// sent in order; a failure partway through leaves everything up to that
/// point on-chain (the vault, approvers list, and any approvers/payees
/// already added are all real and can be inspected/retried individually
/// with the existing single-purpose commands).
fn cmd_create_vault(args: &[String]) {
    let path = registry_path(args);
    let mut reg = registry::load(&path);
    let name = arg_req(args, "--name");
    let owner_name = arg_req(args, "--owner");
    let mint = arg_pubkey(args, "--mint");
    let is_pool = args.iter().any(|a| a == "--pool");
    let daily_limit = arg(args, "--daily-limit")
        .map(|s| s.parse::<u64>().unwrap())
        .unwrap_or(0);
    let approver_names = arg_name_list(args, "--approvers");
    let beneficiary_names = arg_name_list(args, "--beneficiaries");
    if !is_pool && (!approver_names.is_empty() || !beneficiary_names.is_empty()) {
        eprintln!("--approvers / --beneficiaries only apply to --pool vaults");
        std::process::exit(1);
    }

    let payer = arg(args, "--payer")
        .map(|p| {
            read_keypair_file(&p).unwrap_or_else(|e| {
                eprintln!("--payer ({p}): could not read: {e}");
                std::process::exit(1);
            })
        })
        .unwrap_or_else(|| reg.person_keypair(&owner_name));
    let owner = reg.person_keypair(&owner_name);

    let rpc = rpc_client(&rpc_url(args));
    let (vault, _) = vault_pda(owner.pubkey(), is_pool);
    let (beneficiary_list, _) = beneficiary_list_pda(vault);
    let vault_token_account =
        spl_associated_token_account::get_associated_token_address(&vault, &mint);

    let data = family_vault::instruction::InitializeVault {
        is_pool,
        daily_limit,
    }
    .data();
    let accounts = family_vault::accounts::InitializeVault {
        payer: to_anchor(payer.pubkey()),
        owner: to_anchor(owner.pubkey()),
        vault: to_anchor(vault),
        beneficiary_list: to_anchor(beneficiary_list),
        mint: to_anchor(mint),
        vault_token_account: to_anchor(vault_token_account),
        token_program: to_anchor(spl_token::ID),
        associated_token_program: to_anchor(spl_associated_token_account::ID),
        system_program: to_anchor(system_program_id()),
    }
    .to_account_metas(None);

    match send_ix(&rpc, accounts, data, &payer, &[&owner]) {
        Ok(sig) => {
            println!("create-vault ok, signature: {sig}");
            reg.vaults.insert(
                name.clone(),
                VaultEntry {
                    pubkey: vault.to_string(),
                    mint: mint.to_string(),
                    owner: owner_name.clone(),
                },
            );
            registry::save(&path, &reg);
            println!("account '{name}' is open (vault: {vault})");
        }
        Err(e) => {
            eprintln!("create-vault failed: {e}");
            std::process::exit(1);
        }
    }

    if !is_pool {
        return;
    }

    let (approvers, _) = approvers_pda(vault);
    let init_approvers_data = family_vault::instruction::InitializeApprovers {}.data();
    let init_approvers_accounts = family_vault::accounts::InitializeApprovers {
        payer: to_anchor(owner.pubkey()),
        vault: to_anchor(vault),
        approvers: to_anchor(approvers),
        system_program: to_anchor(system_program_id()),
    }
    .to_account_metas(None);
    match send_ix(&rpc, init_approvers_accounts, init_approvers_data, &owner, &[]) {
        Ok(sig) => println!("  approvers list initialized, signature: {sig}"),
        Err(e) => {
            eprintln!("init-approvers (during create-vault) failed: {e}");
            std::process::exit(1);
        }
    }

    for approver_name in &approver_names {
        let address = reg.person_pubkey(approver_name);
        let data = family_vault::instruction::AddTrustedApprover {
            address: to_anchor(address),
        }
        .data();
        let accounts = family_vault::accounts::AddTrustedApprover {
            signer: to_anchor(owner.pubkey()),
            vault: to_anchor(vault),
            approvers: to_anchor(approvers),
            existing_approver: None,
        }
        .to_account_metas(None);
        match send_ix(&rpc, accounts, data, &owner, &[]) {
            Ok(sig) => println!("  approver '{approver_name}' added, signature: {sig}"),
            Err(e) => {
                eprintln!("add-approver '{approver_name}' (during create-vault) failed: {e}");
                std::process::exit(1);
            }
        }
    }

    for beneficiary_name in &beneficiary_names {
        let address = reg.person_pubkey(beneficiary_name);
        let data = family_vault::instruction::AddBeneficiary {
            name: beneficiary_name.clone(),
            address: to_anchor(address),
        }
        .data();
        let accounts = family_vault::accounts::AddBeneficiary {
            signer: to_anchor(owner.pubkey()),
            vault: to_anchor(vault),
            beneficiary_list: to_anchor(beneficiary_list),
        }
        .to_account_metas(None);
        match send_ix(&rpc, accounts, data, &owner, &[]) {
            Ok(sig) => println!("  payee '{beneficiary_name}' added, signature: {sig}"),
            Err(e) => {
                eprintln!("add-beneficiary '{beneficiary_name}' (during create-vault) failed: {e}");
                std::process::exit(1);
            }
        }
    }

    println!("account '{name}' fully opened (pool vault, {} approver(s), {} payee(s))", approver_names.len(), beneficiary_names.len());
}

fn cmd_deposit(args: &[String]) {
    let path = registry_path(args);
    let reg = registry::load(&path);
    let from_name = arg_req(args, "--from");
    let to_name = arg_req(args, "--to");
    let amount_whole_units = arg_u64(args, "--amount");

    let depositor = reg.person_keypair(&from_name);
    let vault = reg.vault_pubkey(&to_name);

    let rpc = rpc_client(&rpc_url(args));
    let vault_bytes = rpc.get_account_data(&vault).unwrap_or_else(|e| {
        eprintln!("could not fetch account '{to_name}': {e}");
        std::process::exit(1);
    });
    let vault_data = family_vault::state::Vault::try_deserialize(&mut &vault_bytes[..])
        .unwrap_or_else(|e| {
            eprintln!("could not decode account '{to_name}': {e}");
            std::process::exit(1);
        });
    let vault_token_account = to_client(vault_data.vault_token_account);
    let vta_bytes = rpc
        .get_account_data(&vault_token_account)
        .unwrap_or_else(|e| {
            eprintln!("could not fetch account '{to_name}' token account: {e}");
            std::process::exit(1);
        });
    let mint = spl_token::state::Account::unpack(&vta_bytes)
        .unwrap_or_else(|e| {
            eprintln!("could not decode account '{to_name}' token account: {e}");
            std::process::exit(1);
        })
        .mint;
    let amount = amount_whole_units * 10u64.pow(fetch_mint_decimals(&rpc, &mint));

    let depositor_token_account =
        spl_associated_token_account::get_associated_token_address(&depositor.pubkey(), &mint);

    let data = family_vault::instruction::Deposit { amount }.data();
    let accounts = family_vault::accounts::Deposit {
        depositor: to_anchor(depositor.pubkey()),
        depositor_token_account: to_anchor(depositor_token_account),
        vault_token_account: to_anchor(vault_token_account),
        vault: to_anchor(vault),
        token_program: to_anchor(spl_token::ID),
    }
    .to_account_metas(None);

    match send_ix(&rpc, accounts, data, &depositor, &[]) {
        Ok(sig) => println!("deposit ok ({from_name} -> {to_name}), signature: {sig}"),
        Err(e) => {
            eprintln!("deposit failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Bank-style "send money": register the recipient as a beneficiary
/// first (see `cmd_add_beneficiary`) — the program only ever pays out
/// to its closed address book, by design (see the parent family-vault
/// design spec's "closed address book" anti-scam rationale).
fn cmd_add_beneficiary(args: &[String]) {
    let path = registry_path(args);
    let reg = registry::load(&path);
    let vault_name = arg_req(args, "--vault");
    let person_name = arg_req(args, "--person");

    let vault_entry = reg.vault(&vault_name);
    let vault = vault_entry.pubkey.parse::<Pubkey>().unwrap();
    let owner_name = vault_entry.owner.clone();
    let signer = reg.person_keypair(&owner_name);
    let address = reg.person_pubkey(&person_name);
    let (beneficiary_list, _) = beneficiary_list_pda(vault);

    let rpc = rpc_client(&rpc_url(args));
    let data = family_vault::instruction::AddBeneficiary {
        name: person_name.clone(),
        address: to_anchor(address),
    }
    .data();
    let accounts = family_vault::accounts::AddBeneficiary {
        signer: to_anchor(signer.pubkey()),
        vault: to_anchor(vault),
        beneficiary_list: to_anchor(beneficiary_list),
    }
    .to_account_metas(None);

    match send_ix(&rpc, accounts, data, &signer, &[]) {
        Ok(sig) => {
            println!("added '{person_name}' as a payee on account '{vault_name}', signature: {sig}")
        }
        Err(e) => {
            eprintln!("add-beneficiary failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Two-approver fast lane: adds a payee immediately, skipping the normal
/// 48h cooldown — see `family_vault::instructions::add_beneficiary_urgent`
/// for why this needs two distinct trusted approvers, not the owner.
fn cmd_add_beneficiary_urgent(args: &[String]) {
    let path = registry_path(args);
    let reg = registry::load(&path);
    let vault_name = arg_req(args, "--vault");
    let person_name = arg_req(args, "--person");
    let approver1_name = arg_req(args, "--approver1");
    let approver2_name = arg_req(args, "--approver2");

    let vault_entry = reg.vault(&vault_name);
    let vault = vault_entry.pubkey.parse::<Pubkey>().unwrap();
    let approver_one = reg.person_keypair(&approver1_name);
    let approver_two = reg.person_keypair(&approver2_name);
    let address = reg.person_pubkey(&person_name);
    let (approvers, _) = approvers_pda(vault);
    let (beneficiary_list, _) = beneficiary_list_pda(vault);

    let rpc = rpc_client(&rpc_url(args));
    let data = family_vault::instruction::AddBeneficiaryUrgent {
        name: person_name.clone(),
        address: to_anchor(address),
    }
    .data();
    let accounts = family_vault::accounts::AddBeneficiaryUrgent {
        approver_one: to_anchor(approver_one.pubkey()),
        approver_two: to_anchor(approver_two.pubkey()),
        vault: to_anchor(vault),
        approvers: to_anchor(approvers),
        beneficiary_list: to_anchor(beneficiary_list),
    }
    .to_account_metas(None);

    match send_ix(&rpc, accounts, data, &approver_one, &[&approver_two]) {
        Ok(sig) => println!(
            "urgently added '{person_name}' as a payee on account '{vault_name}' (immediately active), signature: {sig}"
        ),
        Err(e) => {
            eprintln!("add-beneficiary-urgent failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Bank-style "send money from account to account". The recipient must
/// already be an approved payee (`add-beneficiary`) — the program
/// enforces this on-chain regardless of what this tool sends.
fn cmd_send(args: &[String]) {
    let path = registry_path(args);
    let reg = registry::load(&path);
    let from_name = arg_req(args, "--from");
    let to_name = arg_req(args, "--to");
    let amount_whole_units = arg_u64(args, "--amount");

    let vault_entry = reg.vault(&from_name);
    let vault = vault_entry.pubkey.parse::<Pubkey>().unwrap();
    let owner_name = vault_entry.owner.clone();
    let signer = reg.person_keypair(&owner_name);
    let recipient = reg.person_pubkey(&to_name);

    let rpc = rpc_client(&rpc_url(args));
    let vault_bytes = rpc.get_account_data(&vault).unwrap_or_else(|e| {
        eprintln!("could not fetch account '{from_name}': {e}");
        std::process::exit(1);
    });
    let vault_data = family_vault::state::Vault::try_deserialize(&mut &vault_bytes[..])
        .unwrap_or_else(|e| {
            eprintln!("could not decode account '{from_name}': {e}");
            std::process::exit(1);
        });
    let vault_token_account = to_client(vault_data.vault_token_account);
    let (beneficiary_list, _) = beneficiary_list_pda(vault);
    let vta_bytes = rpc
        .get_account_data(&vault_token_account)
        .unwrap_or_else(|e| {
            eprintln!("could not fetch account '{from_name}' token account: {e}");
            std::process::exit(1);
        });
    let mint = spl_token::state::Account::unpack(&vta_bytes).unwrap().mint;
    let recipient_token_account =
        spl_associated_token_account::get_associated_token_address(&recipient, &mint);
    let amount = amount_whole_units * 10u64.pow(fetch_mint_decimals(&rpc, &mint));

    let data = family_vault::instruction::Withdraw { amount }.data();
    let accounts = family_vault::accounts::Withdraw {
        signer: to_anchor(signer.pubkey()),
        vault_token_account: to_anchor(vault_token_account),
        recipient_token_account: to_anchor(recipient_token_account),
        vault: to_anchor(vault),
        beneficiary_list: to_anchor(beneficiary_list),
        token_program: to_anchor(spl_token::ID),
    }
    .to_account_metas(None);

    match send_ix(&rpc, accounts, data, &signer, &[]) {
        Ok(sig) => println!("sent {amount} from '{from_name}' to '{to_name}', signature: {sig}"),
        Err(e) => {
            eprintln!("send failed: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_init_approvers(args: &[String]) {
    let rpc = rpc_client(&rpc_url(args));
    let payer = arg_keypair(args, "--payer");
    let vault = arg_pubkey(args, "--vault");
    let (approvers, _) = approvers_pda(vault);

    let data = family_vault::instruction::InitializeApprovers {}.data();
    let accounts = family_vault::accounts::InitializeApprovers {
        payer: to_anchor(payer.pubkey()),
        vault: to_anchor(vault),
        approvers: to_anchor(approvers),
        system_program: to_anchor(system_program_id()),
    }
    .to_account_metas(None);

    match send_ix(&rpc, accounts, data, &payer, &[]) {
        Ok(sig) => {
            println!("init-approvers ok, signature: {sig}");
            println!("approvers: {approvers}");
        }
        Err(e) => {
            eprintln!("init-approvers failed: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_add_approver(args: &[String]) {
    let rpc = rpc_client(&rpc_url(args));
    let signer = arg_keypair(args, "--signer");
    let vault = arg_pubkey(args, "--vault");
    let address = arg_pubkey(args, "--address");
    let existing_approver_kp =
        arg(args, "--existing-approver").map(|_| arg_keypair(args, "--existing-approver"));
    let (approvers, _) = approvers_pda(vault);

    let data = family_vault::instruction::AddTrustedApprover {
        address: to_anchor(address),
    }
    .data();
    let accounts = family_vault::accounts::AddTrustedApprover {
        signer: to_anchor(signer.pubkey()),
        vault: to_anchor(vault),
        approvers: to_anchor(approvers),
        existing_approver: existing_approver_kp.as_ref().map(|k| to_anchor(k.pubkey())),
    }
    .to_account_metas(None);

    let extra: Vec<&Keypair> = existing_approver_kp.iter().collect();
    match send_ix(&rpc, accounts, data, &signer, &extra) {
        Ok(sig) => println!("add-approver ok, signature: {sig}"),
        Err(e) => {
            eprintln!("add-approver failed: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_request_withdrawal(args: &[String]) {
    let rpc = rpc_client(&rpc_url(args));
    let signer = arg_keypair(args, "--signer");
    let vault = arg_pubkey(args, "--vault");
    let recipient = arg_pubkey(args, "--recipient");
    let amount = arg_u64(args, "--amount");
    let nonce = arg_u64(args, "--nonce");
    let (beneficiary_list, _) = beneficiary_list_pda(vault);
    let (pending_withdrawal, _) = request_pda(vault, nonce);

    let vault_bytes = rpc.get_account_data(&vault).unwrap();
    let vault_data = family_vault::state::Vault::try_deserialize(&mut &vault_bytes[..]).unwrap();
    let vta_bytes = rpc
        .get_account_data(&to_client(vault_data.vault_token_account))
        .unwrap();
    let mint = spl_token::state::Account::unpack(&vta_bytes).unwrap().mint;
    let recipient_token_account =
        spl_associated_token_account::get_associated_token_address(&recipient, &mint);

    let data = family_vault::instruction::RequestWithdrawal { amount, nonce }.data();
    let accounts = family_vault::accounts::RequestWithdrawal {
        signer: to_anchor(signer.pubkey()),
        vault: to_anchor(vault),
        beneficiary_list: to_anchor(beneficiary_list),
        recipient_token_account: to_anchor(recipient_token_account),
        pending_withdrawal: to_anchor(pending_withdrawal),
        system_program: to_anchor(system_program_id()),
    }
    .to_account_metas(None);

    match send_ix(&rpc, accounts, data, &signer, &[]) {
        Ok(sig) => {
            println!("request-withdrawal ok, signature: {sig}");
            println!("pending_withdrawal: {pending_withdrawal}");
        }
        Err(e) => {
            eprintln!("request-withdrawal failed: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_approve_withdrawal(args: &[String]) {
    let rpc = rpc_client(&rpc_url(args));
    let signer = arg_keypair(args, "--signer");
    let vault = arg_pubkey(args, "--vault");
    let nonce = arg_u64(args, "--nonce");
    let (approvers, _) = approvers_pda(vault);
    let (pending_withdrawal, _) = request_pda(vault, nonce);

    let pw_bytes = rpc.get_account_data(&pending_withdrawal).unwrap();
    let pw = family_vault::state::PendingWithdrawal::try_deserialize(&mut &pw_bytes[..]).unwrap();
    let recipient = to_client(pw.to);

    let vault_bytes = rpc.get_account_data(&vault).unwrap();
    let vault_data = family_vault::state::Vault::try_deserialize(&mut &vault_bytes[..]).unwrap();
    let vault_token_account = to_client(vault_data.vault_token_account);
    let vta_bytes = rpc.get_account_data(&vault_token_account).unwrap();
    let mint = spl_token::state::Account::unpack(&vta_bytes).unwrap().mint;
    let recipient_token_account =
        spl_associated_token_account::get_associated_token_address(&recipient, &mint);

    let data = family_vault::instruction::ApproveWithdrawal {}.data();
    let accounts = family_vault::accounts::ApproveWithdrawal {
        signer: to_anchor(signer.pubkey()),
        pending_withdrawal: to_anchor(pending_withdrawal),
        approvers: to_anchor(approvers),
        vault_token_account: to_anchor(vault_token_account),
        recipient_token_account: to_anchor(recipient_token_account),
        vault: to_anchor(vault),
        token_program: to_anchor(spl_token::ID),
    }
    .to_account_metas(None);

    match send_ix(&rpc, accounts, data, &signer, &[]) {
        Ok(sig) => println!("approve-withdrawal ok, signature: {sig}"),
        Err(e) => {
            eprintln!("approve-withdrawal failed: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_request_rotation(args: &[String]) {
    let rpc = rpc_client(&rpc_url(args));
    let signer = arg_keypair(args, "--signer");
    let vault = arg_pubkey(args, "--vault");
    let new_owner = arg_pubkey(args, "--new-owner");
    let nonce = arg_u64(args, "--nonce");
    let (approvers, _) = approvers_pda(vault);
    let (rotation, _) = rotation_pda(vault, nonce);

    let data = family_vault::instruction::RequestOwnerRotation {
        new_owner: to_anchor(new_owner),
        nonce,
    }
    .data();
    let accounts = family_vault::accounts::RequestOwnerRotation {
        signer: to_anchor(signer.pubkey()),
        vault: to_anchor(vault),
        approvers: to_anchor(approvers),
        rotation: to_anchor(rotation),
        system_program: to_anchor(system_program_id()),
    }
    .to_account_metas(None);

    match send_ix(&rpc, accounts, data, &signer, &[]) {
        Ok(sig) => {
            println!("request-rotation ok, signature: {sig}");
            println!("rotation: {rotation}");
        }
        Err(e) => {
            eprintln!("request-rotation failed: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_cancel_rotation(args: &[String]) {
    let rpc = rpc_client(&rpc_url(args));
    let signer = arg_keypair(args, "--signer");
    let vault = arg_pubkey(args, "--vault");
    let nonce = arg_u64(args, "--nonce");
    let (approvers, _) = approvers_pda(vault);
    let (rotation, _) = rotation_pda(vault, nonce);

    let data = family_vault::instruction::CancelOwnerRotation {}.data();
    let accounts = family_vault::accounts::CancelOwnerRotation {
        signer: to_anchor(signer.pubkey()),
        vault: to_anchor(vault),
        approvers: to_anchor(approvers),
        rotation: to_anchor(rotation),
    }
    .to_account_metas(None);

    match send_ix(&rpc, accounts, data, &signer, &[]) {
        Ok(sig) => println!("cancel-rotation ok, signature: {sig}"),
        Err(e) => {
            eprintln!("cancel-rotation failed: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_execute_rotation(args: &[String]) {
    let rpc = rpc_client(&rpc_url(args));
    let signer = arg_keypair(args, "--signer");
    let vault = arg_pubkey(args, "--vault");
    let nonce = arg_u64(args, "--nonce");
    let (rotation, _) = rotation_pda(vault, nonce);

    let data = family_vault::instruction::ExecuteOwnerRotation {}.data();
    let accounts = family_vault::accounts::ExecuteOwnerRotation {
        signer: to_anchor(signer.pubkey()),
        vault: to_anchor(vault),
        rotation: to_anchor(rotation),
    }
    .to_account_metas(None);

    match send_ix(&rpc, accounts, data, &signer, &[]) {
        Ok(sig) => println!("execute-rotation ok, signature: {sig}"),
        Err(e) => {
            eprintln!("execute-rotation failed: {e}");
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!(
        "admin-cli <command> [flags]\n\n\
         everyday commands (bank-style, use names — see the registry below):\n\
         \x20 register-person   --name <n> (--keypair <path> | --pubkey <addr>)\n\
         \x20 register-vault    --name <n> --pubkey <addr> --mint <addr> --owner <person-name>\n\
         \x20 create-vault      --name <n> --owner <person-name> --mint <addr> [--pool] [--daily-limit <u64>] [--payer <path>] [--approvers p1,p2,...] [--beneficiaries b1,b2,...]\n\
         \x20                   (--approvers/--beneficiaries require --pool; opens approvers list + adds each in one call)\n\
         \x20 add-beneficiary   --vault <n> --person <person-name>\n\
         \x20 add-beneficiary-urgent --vault <n> --person <person-name> --approver1 <name> --approver2 <name>  (two distinct approvers, skips the 48h cooldown)\n\
         \x20 deposit           --from <person-name> --to <vault-name> --amount <whole units, e.g. 10>\n\
         \x20 send              --from <vault-name> --to <person-name> --amount <whole units, e.g. 10>\n\n\
         advanced / raw-pubkey commands (approvers, requests, rotation):\n\
         \x20 init-approvers    --payer <kp> --vault <pk>\n\
         \x20 add-approver      --signer <kp> --vault <pk> --address <pk> [--existing-approver <kp>]\n\
         \x20 request-withdrawal --signer <kp> --vault <pk> --recipient <pk> --amount <u64 base units> --nonce <u64>\n\
         \x20 approve-withdrawal --signer <kp> --vault <pk> --nonce <u64>\n\
         \x20 request-rotation  --signer <kp> --vault <pk> --new-owner <pk> --nonce <u64>\n\
         \x20 cancel-rotation   --signer <kp> --vault <pk> --nonce <u64>\n\
         \x20 execute-rotation  --signer <kp> --vault <pk> --nonce <u64>\n\n\
         all commands accept --rpc-url <url> (default https://api.devnet.solana.com)\n\
         everyday commands also accept --registry <path> (default ~/.family-vault/accounts.json)"
    );
}

fn main() {
    let all_args: Vec<String> = std::env::args().collect();
    let Some(command) = all_args.get(1) else {
        print_usage();
        std::process::exit(1);
    };
    let rest = &all_args[2..];

    match command.as_str() {
        "register-person" => cmd_register_person(rest),
        "register-vault" => cmd_register_vault(rest),
        "create-vault" => cmd_create_vault(rest),
        "deposit" => cmd_deposit(rest),
        "add-beneficiary" => cmd_add_beneficiary(rest),
        "add-beneficiary-urgent" => cmd_add_beneficiary_urgent(rest),
        "send" => cmd_send(rest),
        "init-approvers" => cmd_init_approvers(rest),
        "add-approver" => cmd_add_approver(rest),
        "request-withdrawal" => cmd_request_withdrawal(rest),
        "approve-withdrawal" => cmd_approve_withdrawal(rest),
        "request-rotation" => cmd_request_rotation(rest),
        "cancel-rotation" => cmd_cancel_rotation(rest),
        "execute-rotation" => cmd_execute_rotation(rest),
        _ => {
            print_usage();
            std::process::exit(1);
        }
    }
}
