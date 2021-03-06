// Copyright (C) 2020 Second State.
// This file is part of Pallet-SSVM.

// Pallet-SSVM is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.

// Pallet-SSVM is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::{AccountCodes, Accounts, Event, Module, Trait};
use codec::{Decode, Encode};
use frame_support::storage::StorageMap;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use sp_core::{H160, H256, U256};
use sp_std::marker::PhantomData;
use sp_std::vec::Vec;
#[cfg(feature = "std")]
use ssvm::host::HostInterface;
#[cfg(feature = "std")]
use ssvm::types::{Address, Bytes, Bytes32, CallKind, StatusCode, StorageStatus, ADDRESS_LENGTH};

#[derive(Clone, Eq, PartialEq, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
/// Ethereum account nonce, balance and code. Used by storage.
pub struct Account {
    /// Account nonce.
    pub nonce: U256,
    /// Account balance.
    pub balance: U256,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
/// Ethereum log. Used for `deposit_event`.
pub struct Log {
    /// Source address of the log.
    pub address: H160,
    /// Topics of the log.
    pub topics: Vec<H256>,
    /// Byte array data of the log.
    pub data: Vec<u8>,
}

pub fn create_address(caller: H160, nonce: U256) -> H160 {
    let mut stream = rlp::RlpStream::new_list(2);
    stream.append(&caller);
    stream.append(&nonce);
    H256::from_slice(Keccak256::digest(&stream.out()).as_slice()).into()
}

pub struct TxContext {
    tx_gas_price: U256,
    tx_origin: H160,
    block_coinbase: H160,
    block_number: i64,
    block_timestamp: i64,
    block_gas_limit: i64,
    block_difficulty: U256,
    chain_id: U256,
}

impl TxContext {
    pub fn new(
        tx_gas_price: U256,
        tx_origin: H160,
        block_coinbase: H160,
        block_number: i64,
        block_timestamp: i64,
        block_gas_limit: i64,
        block_difficulty: U256,
        chain_id: U256,
    ) -> Self {
        Self {
            tx_gas_price,
            tx_origin,
            block_coinbase,
            block_number,
            block_timestamp,
            block_gas_limit,
            block_difficulty,
            chain_id,
        }
    }
}

#[cfg(feature = "std")]
pub struct HostContext<T> {
    tx_context: TxContext,
    _marker: PhantomData<T>,
}

#[cfg(feature = "std")]
impl<T> HostContext<T> {
    pub fn new(tx_context: TxContext) -> Self {
        Self {
            tx_context,
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "std")]
impl<T: Trait> HostInterface for HostContext<T> {
    fn account_exists(&mut self, _addr: &[u8; 20]) -> bool {
        true
    }
    fn get_storage(&mut self, address: &Address, key: &Bytes32) -> Bytes32 {
        let ret =
            Module::<T>::get_storage(H160::from(address.to_owned()), H256::from(key.to_owned()));
        ret.to_fixed_bytes()
    }
    fn set_storage(&mut self, address: &Address, key: &Bytes32, value: &Bytes32) -> StorageStatus {
        Module::<T>::set_storage(
            H160::from(address.to_owned()),
            H256::from(key.to_owned()),
            H256::from(value.to_owned()),
        );
        StorageStatus::EVMC_STORAGE_MODIFIED
    }
    fn get_balance(&mut self, address: &Address) -> Bytes32 {
        let balance = Accounts::get(H160::from(address.to_owned())).balance;
        balance.into()
    }
    fn get_code_size(&mut self, address: &Address) -> usize {
        AccountCodes::decode_len(H160::from(address)).unwrap_or(0)
    }
    fn get_code_hash(&mut self, address: &Address) -> Bytes32 {
        H256::from_slice(Keccak256::digest(&AccountCodes::get(H160::from(address))).as_slice())
            .into()
    }
    fn copy_code(
        &mut self,
        _addr: &Address,
        _offset: &usize,
        _buffer_data: &*mut u8,
        _buffer_size: &usize,
    ) -> usize {
        0
    }
    fn selfdestruct(&mut self, _addr: &Address, _beneficiary: &Address) {}
    fn get_tx_context(&mut self) -> (Bytes32, Address, Address, i64, i64, i64, Bytes32) {
        (
            self.tx_context.tx_gas_price.into(),
            self.tx_context.tx_origin.to_fixed_bytes(),
            self.tx_context.block_coinbase.to_fixed_bytes(),
            self.tx_context.block_number,
            self.tx_context.block_timestamp,
            self.tx_context.block_gas_limit,
            self.tx_context.block_difficulty.into(),
        )
    }
    fn get_block_hash(&mut self, block_number: i64) -> Bytes32 {
        let number = U256::from(block_number);
        if number > U256::from(u32::max_value()) {
            H256::default().into()
        } else {
            let number = T::BlockNumber::from(number.as_u32());
            H256::from_slice(frame_system::Module::<T>::block_hash(number).as_ref()).into()
        }
    }
    fn emit_log(&mut self, address: &Address, topics: &Vec<Bytes32>, data: &Bytes) {
        Module::<T>::deposit_event(Event::Log(Log {
            address: H160::from(address.to_owned()),
            topics: topics
                .iter()
                .map(|b32| H256::from(b32))
                .collect::<Vec<H256>>(),
            data: data.to_vec(),
        }));
    }
    fn call(
        &mut self,
        _kind: CallKind,
        _destination: &Address,
        _sender: &Address,
        _value: &Bytes32,
        _input: &[u8],
        _gas: i64,
        _depth: i32,
        _is_static: bool,
    ) -> (Vec<u8>, i64, Address, StatusCode) {
        let (output, gas_left, status_code) = Module::<T>::execute_ssvm(
            _sender.into(),
            _destination.into(),
            _value.into(),
            _input.to_vec(),
            _gas as u32,
            self.tx_context.tx_gas_price.into(),
            Accounts::get(H160::from(_sender)).nonce,
            _kind,
        )
        .unwrap();
        return (output, gas_left, [0u8; ADDRESS_LENGTH], status_code);
    }
}
