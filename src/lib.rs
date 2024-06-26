mod abi;

use substreams::hex;
use substreams::scalar::BigInt;
use substreams::store::StoreAdd;
use substreams::store::StoreAddBigInt;
use substreams::store::StoreGet;
use substreams::store::StoreGetBigInt;
use substreams::store::StoreNew;
use substreams::store::StoreSet;
use substreams::store::StoreSetBigInt;
use substreams::Hex;
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::Event;

const WETH_USDC_POOL_ADDR: [u8; 20] = hex!("88e6a0c2ddd26feeb64f039a2c41296fcb3f5640");

// Should be in it's own mapper, for simplicity, we extract from full block directly
enum PoolEvent {
    Mint {
        ordinal: u64,
        amount0: BigInt,
        amount1: BigInt,
        tx: String,
    },
    Burn {
        ordinal: u64,
        amount0: BigInt,
        amount1: BigInt,
        tx: String,
    },
    Swap {
        ordinal: u64,
        amount0: BigInt,
        amount1: BigInt,
        tx: String,
    },
}

#[substreams::handlers::store]
fn store_mint_burn_liquidity(blk: Block, s: StoreAddBigInt) {
    for event in block_to_events(blk) {
        match event {
            PoolEvent::Mint {
                ordinal,
                amount0,
                amount1,
                tx,
            } => {
                substreams::log::info!("Mint at tx {}", tx);
                s.add(ordinal, "amount0", &amount0);
                s.add(ordinal, "amount1", &amount1);
            }
            PoolEvent::Burn {
                ordinal,
                amount0,
                amount1,
                tx,
            } => {
                substreams::log::info!("Burn at tx {}", tx);
                s.add(ordinal, "amount0", &amount0.neg());
                s.add(ordinal, "amount1", &amount1.neg());
            }
            PoolEvent::Swap { .. } => {
                // No swap for now
            }
        }
    }
}

#[substreams::handlers::store]
fn store_swap_liquidity(blk: Block, s: StoreSetBigInt) {
    for event in block_to_events(blk) {
        if let PoolEvent::Swap {
            ordinal,
            amount0,
            amount1,
            tx,
        } = event
        {
            substreams::log::info!("Swap at tx {}", tx);
            s.set(ordinal, "amount0", &amount0);
            s.set(ordinal, "amount1", &amount1);
        }
    }
}

#[substreams::handlers::map]
fn map_output(mint_burn: StoreGetBigInt, swap: StoreGetBigInt) -> Option<()> {
    let mint_burn_last_0 = mint_burn.get_last("amount0").unwrap_or_default();
    let swap_last_0 = swap.get_last("amount0").unwrap_or_default();

    let mint_burn_last_1 = mint_burn.get_last("amount1").unwrap_or_default();
    let swap_last_1 = swap.get_last("amount1").unwrap_or_default();

    substreams::log::info!(
        "Amount0 at end of block: {}\nAmount1 at end of block: {}",
        (swap_last_0 + mint_burn_last_0),
        (swap_last_1 + mint_burn_last_1),
    );

    Some(())
}

fn block_to_events(blk: Block) -> Vec<PoolEvent> {
    use abi::pool::events::{Burn, Mint, Swap};

    let events = blk
        .logs()
        .filter_map(|log_view| {
            if log_view.address() != WETH_USDC_POOL_ADDR {
                return None;
            }

            if let Some(mint) = Mint::match_and_decode(log_view.log) {
                Some(PoolEvent::Mint {
                    ordinal: log_view.ordinal(),
                    amount0: mint.amount0,
                    amount1: mint.amount1,
                    tx: Hex(&log_view.receipt.transaction.hash).to_string(),
                })
            } else if let Some(burn) = Burn::match_and_decode(log_view.log) {
                Some(PoolEvent::Burn {
                    ordinal: log_view.ordinal(),
                    amount0: burn.amount0,
                    amount1: burn.amount1,
                    tx: Hex(&log_view.receipt.transaction.hash).to_string(),
                })
            } else if let Some(swap) = Swap::match_and_decode(log_view.log) {
                Some(PoolEvent::Swap {
                    ordinal: log_view.ordinal(),
                    amount0: swap.amount0,
                    amount1: swap.amount1,
                    tx: Hex(&log_view.receipt.transaction.hash).to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    return events;
}
