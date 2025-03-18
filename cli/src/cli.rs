use crate::commands::l2;
use crate::common::{AddressOpts, HashOpts};
use crate::utils::{parse_address_opts, parse_hash_opts, parse_hex, parse_message};
use crate::{
    commands::autocomplete,
    common::{CallArgs, DeployArgs, SendArgs, TransferArgs},
};
use clap::{Parser, Subcommand};
use ethrex_common::{Address, Bytes, H256};
use keccak_hash::keccak;
use rex_sdk::{
    balance_in_eth,
    client::{EthClient, Overrides, eth::get_address_from_secret_key},
    transfer, wait_for_transaction_receipt,
};

pub const VERSION_STRING: &str = env!("CARGO_PKG_VERSION");

pub async fn start() -> eyre::Result<()> {
    let CLI { command } = CLI::parse();
    command.run().await
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Parser)]
#[command(name="rex", author, version=VERSION_STRING, about, long_about = None)]
pub(crate) struct CLI {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    #[clap(
        about = "Get either the account's address from private key, the zero address, or a random address",
        visible_aliases = ["addr", "a"]
    )]
    Address {
        #[arg(value_parser = parse_address_opts, help = "Address options. (random, zero, from-private-key)", long_help = "random - random address, zero - zero address, from-private-key - address from the private key")]
        opts: AddressOpts,
    },
    #[clap(subcommand, about = "Generate shell completion scripts.")]
    Autocomplete(autocomplete::Command),
    #[clap(about = "Get the account's balance info.", visible_aliases = ["bal", "b"])]
    Balance {
        account: Address,
        #[clap(
            long = "token",
            help = "Specify the token address, the ETH is used as default."
        )]
        token_address: Option<Address>,
        #[arg(
            long = "eth",
            required = false,
            default_value_t = false,
            help = "Display the balance in ETH."
        )]
        eth: bool,
        #[arg(default_value = "http://localhost:8545", env = "RPC_URL")]
        rpc_url: String,
    },
    #[clap(about = "Get the current block_number.", visible_alias = "bl")]
    BlockNumber {
        #[arg(default_value = "http://localhost:8545", env = "RPC_URL")]
        rpc_url: String,
    },
    #[clap(about = "Make a call to a contract")]
    Call {
        #[clap(flatten)]
        args: CallArgs,
        #[arg(default_value = "http://localhost:8545", env = "RPC_URL")]
        rpc_url: String,
    },
    #[clap(about = "Get the network's chain id.")]
    ChainId {
        #[arg(
            short,
            long,
            default_value_t = false,
            help = "Display the chain id as a hex-string."
        )]
        hex: bool,
        #[arg(default_value = "http://localhost:8545", env = "RPC_URL")]
        rpc_url: String,
    },
    #[clap(about = "Deploy a contract")]
    Deploy {
        #[clap(flatten)]
        args: DeployArgs,
        #[arg(default_value = "http://localhost:8545", env = "RPC_URL")]
        rpc_url: String,
    },
    #[clap(
        about = "Get either the keccak for a given input, the zero hash, the empty string, or a random hash",
        visible_alias = "h"
    )]
    Hash {
        #[arg(value_parser = parse_hash_opts, help = "Hash options. (zero, random, string, input)", long_help = "zero - zero hash, random - random hash, string - empty string hash, input - hash of the input")]
        opts: HashOpts,
    },
    #[clap(subcommand, about = "L2 specific commands.")]
    L2(l2::Command),
    #[clap(about = "Get the account's nonce.", visible_aliases = ["n"])]
    Nonce {
        account: Address,
        #[arg(default_value = "http://localhost:8545", env = "RPC_URL")]
        rpc_url: String,
    },
    #[clap(about = "Get the transaction's receipt.", visible_alias = "r")]
    Receipt {
        tx_hash: H256,
        #[arg(default_value = "http://localhost:8545", env = "RPC_URL")]
        rpc_url: String,
    },
    #[clap(about = "Send a transaction")]
    Send {
        #[clap(flatten)]
        args: SendArgs,
        #[arg(default_value = "http://localhost:8545", env = "RPC_URL")]
        rpc_url: String,
    },
    Signer {
        #[arg(value_parser = parse_message)]
        message: secp256k1::Message,
        #[arg(value_parser = parse_hex)]
        signature: Bytes,
    },
    #[clap(about = "Get the transaction's info.", visible_aliases = ["tx", "t"])]
    Transaction {
        tx_hash: H256,
        #[arg(default_value = "http://localhost:8545", env = "RPC_URL")]
        rpc_url: String,
    },
    #[clap(about = "Transfer funds to another wallet.")]
    Transfer {
        #[clap(flatten)]
        args: TransferArgs,
        #[arg(default_value = "http://localhost:8545", env = "RPC_URL")]
        rpc_url: String,
    },
}

impl Command {
    pub async fn run(self) -> eyre::Result<()> {
        match self {
            Command::L2(cmd) => cmd.run().await?,
            Command::Autocomplete(cmd) => cmd.run()?,
            Command::Balance {
                account,
                token_address,
                eth,
                rpc_url,
            } => {
                if token_address.is_some() {
                    todo!("Handle ERC20 balances")
                }

                let eth_client = EthClient::new(&rpc_url);

                let account_balance = eth_client.get_balance(account).await?;

                println!("{}", balance_in_eth(eth, account_balance));
            }
            Command::BlockNumber { rpc_url } => {
                let eth_client = EthClient::new(&rpc_url);

                let block_number = eth_client.get_block_number().await?;

                println!("{block_number}");
            }
            Command::Transaction { tx_hash, rpc_url } => {
                let eth_client = EthClient::new(&rpc_url);

                let tx = eth_client
                    .get_transaction_by_hash(tx_hash)
                    .await?
                    .ok_or(eyre::Error::msg("Not found"))?;

                println!("{tx}");
            }
            Command::Receipt { tx_hash, rpc_url } => {
                let eth_client = EthClient::new(&rpc_url);

                let receipt = eth_client
                    .get_transaction_receipt(tx_hash)
                    .await?
                    .ok_or(eyre::Error::msg("Not found"))?;

                println!("{:x?}", receipt.tx_info);
            }
            Command::Nonce { account, rpc_url } => {
                let eth_client = EthClient::new(&rpc_url);

                let nonce = eth_client.get_nonce(account).await?;

                println!("{nonce}");
            }
            Command::Address { opts } => {
                let address = match opts {
                    AddressOpts::FromPrivateKey(secret_key) => {
                        get_address_from_secret_key(&secret_key)?
                    }
                    AddressOpts::Random => Address::random(),
                    AddressOpts::Zero => Address::zero(),
                };

                println!("{address:#x}");
            }
            Command::Hash { opts } => {
                let hash = match opts {
                    HashOpts::Input(input) => keccak(&input),
                    HashOpts::Zero => H256::zero(),
                    HashOpts::Random => H256::random(),
                    HashOpts::String => keccak(b""),
                };

                println!("{hash:#x}");
            }
            Command::Signer { message, signature } => {
                let raw_recovery_id = if signature[64] >= 27 {
                    signature[64] - 27
                } else {
                    signature[64]
                };

                let recovery_id = secp256k1::ecdsa::RecoveryId::from_i32(raw_recovery_id as i32)?;

                let signature = secp256k1::ecdsa::RecoverableSignature::from_compact(
                    &signature[..64],
                    recovery_id,
                )?;

                let signer_public_key = signature.recover(&message)?;

                let signer =
                    hex::encode(&keccak(&signer_public_key.serialize_uncompressed()[1..])[12..]);

                println!("0x{signer}");
            }
            Command::Transfer { args, rpc_url } => {
                if args.token_address.is_some() {
                    todo!("Handle ERC20 transfers")
                }

                if args.explorer_url {
                    todo!("Display transaction URL in the explorer")
                }

                let from = get_address_from_secret_key(&args.private_key)?;

                let client = EthClient::new(&rpc_url);

                let tx_hash = transfer(
                    args.amount,
                    from,
                    args.to,
                    args.private_key,
                    &client,
                    Overrides {
                        value: Some(args.amount),
                        nonce: args.nonce,
                        ..Default::default()
                    },
                )
                .await?;

                println!("{tx_hash:#x}");

                if !args.background {
                    wait_for_transaction_receipt(tx_hash, &client, 100).await?;
                }
            }
            Command::Send { args, rpc_url } => {
                if args.explorer_url {
                    todo!("Display transaction URL in the explorer")
                }

                let from = get_address_from_secret_key(&args.private_key)?;

                let client = EthClient::new(&rpc_url);

                let tx = client
                    .build_eip1559_transaction(
                        args.to,
                        from,
                        args.calldata,
                        Overrides {
                            value: Some(args.value),
                            chain_id: args.chain_id,
                            nonce: args.nonce,
                            gas_limit: args.gas_limit,
                            max_fee_per_gas: args.max_fee_per_gas,
                            max_priority_fee_per_gas: args.max_priority_fee_per_gas,
                            from: Some(from),
                            ..Default::default()
                        },
                        10,
                    )
                    .await?;

                let tx_hash = client
                    .send_eip1559_transaction(&tx, &args.private_key)
                    .await?;

                println!("{tx_hash:#x}",);

                if !args.background {
                    wait_for_transaction_receipt(tx_hash, &client, 100).await?;
                }
            }
            Command::Call { args, rpc_url } => {
                if args.explorer_url {
                    todo!("Display transaction URL in the explorer")
                }

                let client = EthClient::new(&rpc_url);

                let result = client
                    .call(
                        args.to,
                        args.calldata,
                        Overrides {
                            from: args.from,
                            value: args.value.into(),
                            gas_limit: args.gas_limit,
                            max_fee_per_gas: args.max_fee_per_gas,
                            ..Default::default()
                        },
                    )
                    .await?;

                println!("{result}");
            }
            Command::Deploy { args, rpc_url } => {
                if args.explorer_url {
                    todo!("Display transaction URL in the explorer")
                }

                let from = get_address_from_secret_key(&args.private_key)?;

                let client = EthClient::new(&rpc_url);

                let (tx_hash, deployed_contract_address) = client
                    .deploy(
                        from,
                        args.private_key,
                        args.bytecode,
                        Overrides {
                            value: args.value.into(),
                            nonce: args.nonce,
                            chain_id: args.chain_id,
                            gas_limit: args.gas_limit,
                            max_fee_per_gas: args.max_fee_per_gas,
                            max_priority_fee_per_gas: args.max_priority_fee_per_gas,
                            ..Default::default()
                        },
                    )
                    .await?;

                println!("Contract deployed in tx: {tx_hash:#x}");
                println!("Contract address: {deployed_contract_address:#x}");

                if !args.background {
                    wait_for_transaction_receipt(tx_hash, &client, 100).await?;
                }
            }
            Command::ChainId { hex, rpc_url } => {
                let eth_client = EthClient::new(&rpc_url);

                let chain_id = eth_client.get_chain_id().await?;

                if hex {
                    println!("{chain_id:#x}");
                } else {
                    println!("{chain_id}");
                }
            }
        };
        Ok(())
    }
}
