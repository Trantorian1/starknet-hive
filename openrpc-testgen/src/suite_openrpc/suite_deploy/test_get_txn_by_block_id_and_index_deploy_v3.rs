use crate::{
    utils::v7::{
        accounts::account::ConnectedAccount,
        contract::factory::ContractFactory,
        endpoints::{errors::RpcError, utils::wait_for_sent_transaction},
        providers::provider::Provider,
    },
    RandomizableAccountsTrait, RunnableTrait,
};
use colored::Colorize;
use rand::{rngs::StdRng, RngCore, SeedableRng};
use starknet_types_core::felt::Felt;
use starknet_types_rpc::{BlockId, DeployTxn, InvokeTxn, MaybePendingBlockWithTxs, Txn};
use tracing::{error, info};

#[derive(Clone, Debug)]
pub struct TestCase {}

impl RunnableTrait for TestCase {
    type Input = super::TestSuiteDeploy;

    async fn run(test_input: &Self::Input) -> Result<Self, RpcError> {
        let factory = ContractFactory::new(
            test_input.declaration_result.class_hash,
            test_input.random_paymaster_account.random_accounts()?,
        );
        let mut salt_buffer = [0u8; 32];
        let mut rng = StdRng::from_entropy();
        rng.fill_bytes(&mut salt_buffer[1..]);

        let invoke_result = factory
            .deploy_v1(vec![], Felt::from_bytes_be(&salt_buffer), true)
            .send()
            .await?;

        wait_for_sent_transaction(
            invoke_result.transaction_hash,
            &test_input.random_paymaster_account.random_accounts()?,
        )
        .await?;

        let block_number = test_input
            .random_paymaster_account
            .provider()
            .block_hash_and_number()
            .await?
            .block_number;

        let block_with_txns = test_input
            .random_paymaster_account
            .provider()
            .get_block_with_txs(BlockId::Number(block_number))
            .await?;

        let txn_index: u64 = match block_with_txns {
            MaybePendingBlockWithTxs::Block(block_with_txs) => block_with_txs
                .transactions
                .iter()
                .position(|tx| tx.transaction_hash == invoke_result.transaction_hash)
                .ok_or_else(|| {
                    RpcError::TransactionNotFound(invoke_result.transaction_hash.to_string())
                })?
                .try_into()
                .map_err(|_| RpcError::TransactionIndexOverflow)?,
            MaybePendingBlockWithTxs::Pending(block_with_txs) => block_with_txs
                .transactions
                .iter()
                .position(|tx| tx.transaction_hash == invoke_result.transaction_hash)
                .ok_or_else(|| {
                    RpcError::TransactionNotFound(invoke_result.transaction_hash.to_string())
                })?
                .try_into()
                .map_err(|_| RpcError::TransactionIndexOverflow)?,
        };

        let txn = test_input
            .random_paymaster_account
            .provider()
            .get_transaction_by_block_id_and_index(BlockId::Number(block_number), txn_index)
            .await?;

        match txn {
            Txn::Deploy(DeployTxn {
                class_hash: _,
                constructor_calldata: _,
                contract_address_salt: _,
                version: _,
            }) => {
                info!(
                    "{} {}",
                    "✓ Rpc test_get_txn_by_block_id_and_index_deploy_v3  DEPLOY TXN COMPATIBLE"
                        .green(),
                    "✓".green()
                );
            }
            Txn::Invoke(InvokeTxn::V1(_)) => {
                info!(
                    "{} {}",
                    "✓ Rpc test_get_txn_by_block_id_and_index_deploy_v3 INVOKEV1 COMPATIBLE"
                        .green(),
                    "✓".green()
                );
            }
            Txn::Invoke(InvokeTxn::V3(_)) => {
                info!(
                    "{} {}",
                    "✓ Rpc test_get_txn_by_block_id_and_index_deploy_v3 INVOKEV3 COMPATIBLE"
                        .green(),
                    "✓".green()
                );
            }
            _ => {
                let error_message = format!("Unexpected transaction response type: {:?}", txn);
                error!(
                    "{} {} {}",
                    "✗ Rpc test_get_txn_by_block_id_and_index_deploy_v3 INCOMPATIBLE:".red(),
                    error_message,
                    "✗".red()
                );
                return Err(RpcError::UnexpectedTxnType(error_message));
            }
        }

        Ok(Self {})
    }
}
