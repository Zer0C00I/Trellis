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

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_client::rpc_client::RpcClient;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::{read_keypair_file, Keypair};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_token::solana_program::program_pack::Pack;

type AnchorPubkey = anchor_lang::prelude::Pubkey;

fn to_anchor(p: Pubkey) -> AnchorPubkey {
    AnchorPubkey::new_from_array(p.to_bytes())
}

fn to_client(p: AnchorPubkey) -> Pubkey {
    Pubkey::new_from_array(p.to_bytes())
}

fn find_pda(seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, program_id)
}

/// `family-vault` only exports its declared id as `anchor_lang::prelude::Pubkey`
/// (via `declare_id!`) — this tool needs it in the client-era `solana_pubkey`
/// type for PDA derivation, RPC calls, and `Instruction::program_id`.
fn program_id() -> Pubkey {
    to_client(family_vault::ID)
}

fn vault_pda(owner: Pubkey, is_pool: bool) -> (Pubkey, u8) {
    let kind: &[u8] = if is_pool { b"pool" } else { b"personal" };
    find_pda(&[b"vault", owner.as_ref(), kind], &program_id())
}

fn beneficiary_list_pda(vault: Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"beneficiaries", vault.as_ref()], &program_id())
}

fn approvers_pda(vault: Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"approvers", vault.as_ref()], &program_id())
}

fn request_pda(vault: Pubkey, nonce: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"request", vault.as_ref(), &nonce.to_le_bytes()],
        &program_id(),
    )
}

fn rotation_pda(vault: Pubkey, nonce: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"rotation", vault.as_ref(), &nonce.to_le_bytes()],
        &program_id(),
    )
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

fn system_program_id() -> Pubkey {
    Pubkey::new_from_array([0u8; 32])
}

fn cmd_init_vault(args: &[String]) {
    let rpc = rpc_client(&rpc_url(args));
    let payer = arg_keypair(args, "--payer");
    let owner = arg_keypair(args, "--owner");
    let mint = arg_pubkey(args, "--mint");
    let is_pool = args.iter().any(|a| a == "--pool");
    let daily_limit = arg(args, "--daily-limit")
        .map(|s| s.parse::<u64>().unwrap())
        .unwrap_or(0);

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
            println!("init-vault ok, signature: {sig}");
            println!("vault: {vault}");
            println!("beneficiary_list: {beneficiary_list}");
            println!("vault_token_account: {vault_token_account}");
        }
        Err(e) => {
            eprintln!("init-vault failed: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_deposit(args: &[String]) {
    let rpc = rpc_client(&rpc_url(args));
    let depositor = arg_keypair(args, "--depositor");
    let vault = arg_pubkey(args, "--vault");
    let amount = arg_u64(args, "--amount");

    let vault_bytes = rpc.get_account_data(&vault).unwrap_or_else(|e| {
        eprintln!("could not fetch vault: {e}");
        std::process::exit(1);
    });
    let vault_data = family_vault::state::Vault::try_deserialize(&mut &vault_bytes[..])
        .unwrap_or_else(|e| {
            eprintln!("could not decode vault: {e}");
            std::process::exit(1);
        });
    let vault_token_account = to_client(vault_data.vault_token_account);
    let vta_bytes = rpc
        .get_account_data(&vault_token_account)
        .unwrap_or_else(|e| {
            eprintln!("could not fetch vault_token_account: {e}");
            std::process::exit(1);
        });
    let mint = spl_token::state::Account::unpack(&vta_bytes)
        .unwrap_or_else(|e| {
            eprintln!("could not decode vault_token_account: {e}");
            std::process::exit(1);
        })
        .mint;

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
        Ok(sig) => println!("deposit ok, signature: {sig}"),
        Err(e) => {
            eprintln!("deposit failed: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_add_beneficiary(args: &[String]) {
    let rpc = rpc_client(&rpc_url(args));
    let signer = arg_keypair(args, "--signer");
    let vault = arg_pubkey(args, "--vault");
    let name = arg_req(args, "--name");
    let address = arg_pubkey(args, "--address");
    let (beneficiary_list, _) = beneficiary_list_pda(vault);

    let data = family_vault::instruction::AddBeneficiary {
        name,
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
        Ok(sig) => println!("add-beneficiary ok, signature: {sig}"),
        Err(e) => {
            eprintln!("add-beneficiary failed: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_withdraw(args: &[String]) {
    let rpc = rpc_client(&rpc_url(args));
    let signer = arg_keypair(args, "--signer");
    let vault = arg_pubkey(args, "--vault");
    let recipient = arg_pubkey(args, "--recipient");
    let amount = arg_u64(args, "--amount");

    let vault_bytes = rpc.get_account_data(&vault).unwrap_or_else(|e| {
        eprintln!("could not fetch vault: {e}");
        std::process::exit(1);
    });
    let vault_data = family_vault::state::Vault::try_deserialize(&mut &vault_bytes[..])
        .unwrap_or_else(|e| {
            eprintln!("could not decode vault: {e}");
            std::process::exit(1);
        });
    let vault_token_account = to_client(vault_data.vault_token_account);
    let (beneficiary_list, _) = beneficiary_list_pda(vault);
    let vta_bytes = rpc
        .get_account_data(&vault_token_account)
        .unwrap_or_else(|e| {
            eprintln!("could not fetch vault_token_account: {e}");
            std::process::exit(1);
        });
    let mint = spl_token::state::Account::unpack(&vta_bytes).unwrap().mint;
    let recipient_token_account =
        spl_associated_token_account::get_associated_token_address(&recipient, &mint);

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
        Ok(sig) => println!("withdraw ok, signature: {sig}"),
        Err(e) => {
            eprintln!("withdraw failed: {e}");
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
         commands:\n\
         \x20 init-vault        --payer <kp> --owner <kp> --mint <pk> [--pool] [--daily-limit <u64>]\n\
         \x20 deposit           --depositor <kp> --vault <pk> --amount <u64>\n\
         \x20 add-beneficiary   --signer <kp> --vault <pk> --name <str> --address <pk>\n\
         \x20 withdraw          --signer <kp> --vault <pk> --recipient <pk> --amount <u64>\n\
         \x20 init-approvers    --payer <kp> --vault <pk>\n\
         \x20 add-approver      --signer <kp> --vault <pk> --address <pk> [--existing-approver <kp>]\n\
         \x20 request-withdrawal --signer <kp> --vault <pk> --recipient <pk> --amount <u64> --nonce <u64>\n\
         \x20 approve-withdrawal --signer <kp> --vault <pk> --nonce <u64>\n\
         \x20 request-rotation  --signer <kp> --vault <pk> --new-owner <pk> --nonce <u64>\n\
         \x20 cancel-rotation   --signer <kp> --vault <pk> --nonce <u64>\n\
         \x20 execute-rotation  --signer <kp> --vault <pk> --nonce <u64>\n\n\
         all commands accept --rpc-url <url> (default https://api.devnet.solana.com)"
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
        "init-vault" => cmd_init_vault(rest),
        "deposit" => cmd_deposit(rest),
        "add-beneficiary" => cmd_add_beneficiary(rest),
        "withdraw" => cmd_withdraw(rest),
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
