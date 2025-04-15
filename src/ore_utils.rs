use bytemuck::{Pod, Zeroable};
use drillx::Solution;
use eore_api::{
    consts::{
        BUS_ADDRESSES, CONFIG_ADDRESS, EPOCH_DURATION, MINT_ADDRESS, PROOF, TOKEN_DECIMALS,
        TREASURY_ADDRESS,
    },
    error::OreError,
    sdk,
    state::{Config, Proof, Treasury},
    ID as ORE_ID,
};
use eore_boost_api::state::{boost_pda, stake_pda};
use num_enum::TryFromPrimitive;
use ore_miner_delegation::{
    impl_instruction_from_bytes, impl_to_bytes, instruction,
    pda::delegated_stake_pda,
    pda::managed_proof_pda,
    state::{DelegatedBoost, DelegatedBoostV2, DelegatedStake},
    utils::AccountDeserializeV1,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::{
    client_error::{ClientError, ClientErrorKind, Result as ClientResult},
    rpc_config::RpcSendTransactionConfig,
};
use solana_program::{instruction::AccountMeta, system_program};
use solana_sdk::{
    account::ReadableAccount,
    clock::Clock,
    commitment_config::CommitmentLevel,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    sysvar,
    transaction::Transaction,
};
use solana_transaction_status::{TransactionConfirmationStatus, UiTransactionEncoding};
use spl_associated_token_account::get_associated_token_address;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{str::FromStr, time::Duration};
pub use steel::AccountDeserialize;
use tracing::{error, info};

pub const ORE_TOKEN_DECIMALS: u8 = TOKEN_DECIMALS;

pub fn get_auth_ix(signer: Pubkey) -> Instruction {
    let proof = proof_pubkey(signer);

    sdk::auth(proof)
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum Instructions {
    OpenManagedProof,
    InitDelegateStake,
    Mine,
    DelegateStake,
    UndelegateStake,
    OpenManagedProofBoost,
    DelegateBoost,
    UndelegateBoost,
    InitDelegateBoost,
    DelegateBoostV2,
    UndelegateBoostV2,
    InitDelegateBoostV2,
    MigrateDelegateBoostToV2,
    CloseDelegateBoostV2,
    RegisterGlobalBoost,
    RotateGlobalBoost,
    UpdateMiningAuthority,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MineArgs {
    pub digest: [u8; 16],
    pub nonce: [u8; 8],
}

impl Into<Vec<u8>> for Instructions {
    fn into(self) -> Vec<u8> {
        vec![self as u8]
    }
}

impl Instructions {
    pub fn to_vec(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

impl_to_bytes!(MineArgs);
impl_instruction_from_bytes!(MineArgs);
pub fn mine_with_boost(miner: Pubkey, bus: Pubkey, solution: Solution) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let ore_proof_address = eore_api::state::proof_pda(managed_proof_address.0);
    let delegated_stake_address = eore_boost_api::state::stake_pda(miner, miner);
    let boost_config = eore_boost_api::state::config_pda();
    let boost_proof = eore_api::state::proof_pda(boost_config.0);

    let accounts = vec![
        AccountMeta::new(miner, true),
        AccountMeta::new(managed_proof_address.0, false),
        AccountMeta::new(bus, false),
        AccountMeta::new_readonly(eore_api::consts::CONFIG_ADDRESS, false),
        AccountMeta::new(ore_proof_address.0, false),
        AccountMeta::new(delegated_stake_address.0, false),
        AccountMeta::new_readonly(sysvar::slot_hashes::id(), false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(eore_api::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(boost_config.0, false),
        AccountMeta::new(boost_proof.0, false),
    ];

    Instruction {
        program_id: ORE_ID,
        accounts,
        data: [
            Instructions::Mine.to_vec(),
            MineArgs {
                digest: solution.d,
                nonce: solution.n,
            }
            .to_bytes()
            .to_vec(),
        ]
        .concat(),
    }
}
pub fn get_mine_with_global_boost_ix(
    signer: Pubkey,
    solution: Solution,
    bus: usize,
) -> Instruction {
    // mine_with_boost(signer, BUS_ADDRESSES[bus], solution)
    let boost_config_address = eore_boost_api::state::config_pda().0;
    sdk::mine(
        signer,
        signer,
        BUS_ADDRESSES[bus],
        solution,
        boost_config_address,
    )
}

pub fn get_register_ix(signer: Pubkey) -> Instruction {
    sdk::open(signer, signer, signer)
}

pub fn get_reset_ix(signer: Pubkey) -> Instruction {
    eore_api::prelude::reset(signer)
}

pub fn get_claim_ix(signer: Pubkey, beneficiary: Pubkey, claim_amount: u64) -> Instruction {
    sdk::claim(signer, beneficiary, claim_amount)
}

pub fn get_stake_ix(signer: Pubkey, sender: Pubkey, stake_amount: u64) -> Instruction {
    instruction::delegate_stake(signer, sender, stake_amount)
}

pub fn get_ore_mint() -> Pubkey {
    MINT_ADDRESS
}

pub fn get_ore_epoch_duration() -> i64 {
    EPOCH_DURATION
}

pub fn get_ore_decimals() -> u8 {
    TOKEN_DECIMALS
}

pub async fn get_config(client: &RpcClient) -> Result<eore_api::state::Config, String> {
    let data = client.get_account_data(&CONFIG_ADDRESS).await;
    match data {
        Ok(data) => {
            let config = Config::try_from_bytes(&data);
            if let Ok(config) = config {
                return Ok(*config);
            } else {
                return Err("Failed to parse config account".to_string());
            }
        }
        Err(_) => return Err("Failed to get config account".to_string()),
    }
}

pub async fn get_proof_and_config_with_busses(
    client: &RpcClient,
    authority: Pubkey,
) -> (
    Result<Proof, ()>,
    Result<eore_api::state::Config, ()>,
    Result<Vec<Result<eore_api::state::Bus, ()>>, ()>,
) {
    let account_pubkeys = vec![
        proof_pubkey(authority),
        CONFIG_ADDRESS,
        BUS_ADDRESSES[0],
        BUS_ADDRESSES[1],
        BUS_ADDRESSES[2],
        BUS_ADDRESSES[3],
        BUS_ADDRESSES[4],
        BUS_ADDRESSES[5],
        BUS_ADDRESSES[6],
        BUS_ADDRESSES[7],
    ];
    let datas = client.get_multiple_accounts(&account_pubkeys).await;
    if let Ok(datas) = datas {
        let proof = if let Some(data) = &datas[0] {
            Ok(*Proof::try_from_bytes(data.data()).expect("Failed to parse treasury account"))
        } else {
            Err(())
        };

        let treasury_config = if let Some(data) = &datas[1] {
            Ok(*eore_api::state::Config::try_from_bytes(data.data())
                .expect("Failed to parse config account"))
        } else {
            Err(())
        };
        let bus_1 = if let Some(data) = &datas[2] {
            Ok(*eore_api::state::Bus::try_from_bytes(data.data())
                .expect("Failed to parse bus1 account"))
        } else {
            Err(())
        };
        let bus_2 = if let Some(data) = &datas[3] {
            Ok(*eore_api::state::Bus::try_from_bytes(data.data())
                .expect("Failed to parse bus2 account"))
        } else {
            Err(())
        };
        let bus_3 = if let Some(data) = &datas[4] {
            Ok(*eore_api::state::Bus::try_from_bytes(data.data())
                .expect("Failed to parse bus3 account"))
        } else {
            Err(())
        };
        let bus_4 = if let Some(data) = &datas[5] {
            Ok(*eore_api::state::Bus::try_from_bytes(data.data())
                .expect("Failed to parse bus4 account"))
        } else {
            Err(())
        };
        let bus_5 = if let Some(data) = &datas[6] {
            Ok(*eore_api::state::Bus::try_from_bytes(data.data())
                .expect("Failed to parse bus5 account"))
        } else {
            Err(())
        };
        let bus_6 = if let Some(data) = &datas[7] {
            Ok(*eore_api::state::Bus::try_from_bytes(data.data())
                .expect("Failed to parse bus6 account"))
        } else {
            Err(())
        };
        let bus_7 = if let Some(data) = &datas[8] {
            Ok(*eore_api::state::Bus::try_from_bytes(data.data())
                .expect("Failed to parse bus7 account"))
        } else {
            Err(())
        };
        let bus_8 = if let Some(data) = &datas[9] {
            Ok(*eore_api::state::Bus::try_from_bytes(data.data())
                .expect("Failed to parse bus1 account"))
        } else {
            Err(())
        };

        (
            proof,
            treasury_config,
            Ok(vec![bus_1, bus_2, bus_3, bus_4, bus_5, bus_6, bus_7, bus_8]),
        )
    } else {
        (Err(()), Err(()), Err(()))
    }
}

pub async fn send_and_confirm(
    client: &RpcClient,
    tx: Transaction,
    times: u8,
) -> ClientResult<Signature> {
    // Build tx
    let send_cfg = RpcSendTransactionConfig {
        skip_preflight: false,
        preflight_commitment: Some(CommitmentLevel::Confirmed),
        encoding: Some(UiTransactionEncoding::Base64),
        max_retries: Some(0),
        min_context_slot: None,
    };
    // Send transaction
    match client.send_transaction_with_config(&tx, send_cfg).await {
        Ok(sig) => {
            // Confirm transaction
            'confirm: for i in 0..times {
                info!("Confirming transaction 第{}次 {}", i, sig);
                tokio::time::sleep(Duration::from_millis(500)).await;
                match client.get_signature_statuses(&[sig]).await {
                    Ok(signature_statuses) => {
                        for status in signature_statuses.value {
                            if let Some(status) = status {
                                if let Some(err) = status.err {
                                    match err {
                                        // Instruction error
                                        solana_sdk::transaction::TransactionError::InstructionError(_, err) => {
                                            match err {
                                                // Custom instruction error, parse into OreError
                                                solana_program::instruction::InstructionError::Custom(err_code) => {
                                                    match err_code {
                                                        e if e == OreError::NeedsReset as u32 => {
                                                            error!("Needs reset. Retrying...");
                                                            break 'confirm;
                                                        },
                                                        _ => {
                                                            error!("{}", &err.to_string());
                                                            return Err(ClientError {
                                                                request: None,
                                                                kind: ClientErrorKind::Custom(err.to_string()),
                                                            });
                                                        }
                                                    }
                                                },

                                                // Non custom instruction error, return
                                                _ => {
                                                    error!("{}", &err.to_string());
                                                    return Err(ClientError {
                                                        request: None,
                                                        kind: ClientErrorKind::Custom(err.to_string()),
                                                    });
                                                }
                                            }
                                        },

                                        // Non instruction error, return
                                        _ => {
                                            error!("{}", &err.to_string());
                                            return Err(ClientError {
                                                request: None,
                                                kind: ClientErrorKind::Custom(err.to_string()),
                                            });
                                        }
                                    }
                                } else if let Some(confirmation) = status.confirmation_status {
                                    match confirmation {
                                        TransactionConfirmationStatus::Processed => {}
                                        TransactionConfirmationStatus::Confirmed
                                        | TransactionConfirmationStatus::Finalized => {
                                            return Ok(sig);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Handle confirmation errors
                    Err(err) => {
                        error!("{}", &err.kind().to_string());
                    }
                }
            }
            return Err(ClientError {
                request: None,
                kind: ClientErrorKind::Custom("Confirmation timeout".to_string()),
            });
        }

        // Handle submit errors
        Err(err) => {
            error!("failed to send transaction Error: {}", err);
            return Err(ClientError {
                request: None,
                kind: ClientErrorKind::Custom(err.to_string()),
            });
        }
    }
}
pub async fn get_proof(client: &RpcClient, authority: Pubkey) -> Result<Proof, String> {
    let proof_address = proof_pubkey(authority);
    let data = client.get_account_data(&proof_address).await;
    match data {
        Ok(data) => {
            let proof = Proof::try_from_bytes(&data);
            if let Ok(proof) = proof {
                return Ok(*proof);
            } else {
                return Err("Failed to parse proof account".to_string());
            }
        }
        Err(_) => return Err("Failed to get proof account".to_string()),
    }
}

pub fn proof_pubkey(authority: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[PROOF, authority.as_ref()], &ORE_ID).0
}

pub fn treasury_tokens_pubkey() -> Pubkey {
    get_associated_token_address(&TREASURY_ADDRESS, &MINT_ADDRESS)
}

pub async fn get_clock_account(client: &RpcClient) -> Result<Clock, ()> {
    if let Ok(data) = client.get_account_data(&sysvar::clock::ID).await {
        if let Ok(data) = bincode::deserialize::<Clock>(&data) {
            Ok(data)
        } else {
            Err(())
        }
    } else {
        Err(())
    }
}

pub fn get_cutoff(proof: Proof, buffer_time: u64) -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get time")
        .as_secs() as i64;
    proof
        .last_hash_at
        .saturating_add(60)
        .saturating_sub(buffer_time as i64)
        .saturating_sub(now)
}
