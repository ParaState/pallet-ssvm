use crate::sp_api_hidden_includes_decl_storage::hidden_include::StorageMap;
use crate::sp_api_hidden_includes_decl_storage::hidden_include::StorageValue;
use crate::{Accounts, FundOptions};
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::{H160 as Address, U256};
use sp_std::vec::Vec;

#[derive(Clone, Eq, PartialEq, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
/// Fund Options
pub struct Options {
    /// Initial timestamp.
    pub init_timestamp: i64,
    /// Pending round.
    pub pending_round: i64,
    /// Unlocked ticks.
    pub unlocked_ticks: i64,
    /// Speed up fraction.
    pub fraction_round: i64,
    /// Speed up fraction.
    pub fraction_peroid: i64,
}

pub struct FundManager;

fn str2address(s: &'static str) -> Address {
    let hex: Vec<u8> = rustc_hex::FromHex::from_hex(s).unwrap_or_default();
    return Address::from_slice(hex.as_slice());
}

fn str2u256(s: &'static str) -> U256 {
    let hex: Vec<u8> = rustc_hex::FromHex::from_hex(s).unwrap_or_default();
    return U256::from_big_endian(hex.as_slice());
}

impl FundManager {
    // There is a total fixed supply of 21 million ETHs.
    // The blockchain unlocks 525,000 ETHs every month in the first 20 months and
    // the monthly release is cut to 1/2 every 20 months.
    // Here we use 1 round denote 30 days (represent 1 month).

    /// Unlock address. (Alice's address)
    const BENEFICIARY: &'static str = "9621dde636de098b43efb0fa9b61facfe328f99d";
    /// Adjust unlock amount period.
    const PERIOD: i64 = 20;
    /// Ticks means seconds in 30 days.
    const TICKS_IN_ROUND: i64 = 30 * 24 * 3600;
    /// Factor
    const FACTOR: u32 = 2;
    /// Total token amount is 21000000000000000000000000 wei.
    const TOTAL_AMOUNT: &'static str = "115EEC47F6CF7E35000000";

    /// Primary unlock token method
    pub fn try_unlock(timestamp: i64) -> U256 {
        // The beneficiary for unlocked token.
        let beneficiary = str2address(FundManager::BENEFICIARY);

        // The start time to apply unlock token mechanism.
        let init_timestamp = FundOptions::get().init_timestamp;

        // Error handling for two cases.
        // 1. `cargo test` will generate unpredictable timestamp from other module test cases.
        // 2. runtime with some empty fileds in genesis file.
        if timestamp <= init_timestamp || init_timestamp == 0 {
            return U256::from(0);
        }

        // Pending round is point to which not fully unlocked round after last time try_unlock.
        let mut pending_round = FundOptions::get().pending_round;

        // It record that already unlocked ticks in last unlocked round.
        let mut unlocked_ticks = FundOptions::get().unlocked_ticks;

        // Speed up parameters.
        // 1. shorten time of the round (default round is 30 days)
        // 2. shorten cut down period (default period is 20 rounds)
        let fraction_r = FundOptions::get().fraction_round;
        let ticks_in_round = if fraction_r > 1 {
            FundManager::TICKS_IN_ROUND / fraction_r
        } else {
            FundManager::TICKS_IN_ROUND
        };
        let fraction_p = FundOptions::get().fraction_peroid;
        let period = if fraction_p > 1 {
            FundManager::PERIOD / fraction_p
        } else {
            FundManager::PERIOD
        };

        // The funding used to accumulate unlock amount at this time.
        let mut funding = U256::from(0);

        // Expect to unlock to which rounds at this time.
        let expected_round = (timestamp - init_timestamp) / ticks_in_round;

        // The number of times we should decrease unlocks amount cut to 1/2.
        let mut exponent = 0;

        let initial_bucket = str2u256(FundManager::TOTAL_AMOUNT)
            / (U256::from(FundManager::FACTOR) * U256::from(period));
        let mut bucket = initial_bucket;
        let mut tick_bucket = bucket / U256::from(ticks_in_round);
        while expected_round >= pending_round {
            // Reduce duplicate calculate action if need. Only re-calculate each 20 rounds.
            if exponent != pending_round / period {
                exponent = pending_round / period;
                bucket = initial_bucket / U256::from(FundManager::FACTOR).pow(U256::from(exponent));
                tick_bucket = bucket / U256::from(ticks_in_round);
            }
            if expected_round - pending_round >= 1 {
                // Condition 1
                // Increase funding with this round remain amount.
                funding = funding + bucket - (U256::from(unlocked_ticks) * tick_bucket);
                unlocked_ticks = 0;
                pending_round += 1;
            } else {
                // Condition 2
                // Increase funding by this round tick's count * tick_bucket - already unlocked.
                let ticks =
                    timestamp - init_timestamp - (ticks_in_round * pending_round) - unlocked_ticks;
                funding = funding + U256::from(ticks) * tick_bucket;
                unlocked_ticks = (unlocked_ticks + ticks) % ticks_in_round;
                break;
            }
        }

        Accounts::mutate(&beneficiary, |account| {
            account.balance += funding;
        });

        FundOptions::mutate(|v| {
            v.pending_round = pending_round;
            v.unlocked_ticks = unlocked_ticks;
        });

        return funding;
    }
}
