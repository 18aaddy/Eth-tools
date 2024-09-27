use std::{f32::consts::E, str::Bytes};

use ethereum_types::{H160, U256, H256};
use rlp::{Rlp, Decodable, DecoderError};
use crate::utils;

#[derive(Debug)]
enum TransactionType {
    Legacy,
    EIP2930,
    EIP1559,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessListItem(H160, Vec<H256>);

impl Decodable for AccessListItem {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        if rlp.item_count()? != 2 {
            return Err(DecoderError::RlpIncorrectListLen);
        }
        Ok(AccessListItem(
            rlp.val_at(0)?,
            rlp.list_at(1)?,
        ))
    }
}

#[derive(Debug)]
pub struct AccessList(pub Vec<AccessListItem>);

impl Decodable for AccessList {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        let vec: Vec<AccessListItem> = rlp.as_list()?;
        Ok(AccessList(vec))
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct Transaction {
    transaction_type: TransactionType,
    chain_id: Option<u64>,
    nonce: U256,
    gas_price: Option<U256>,
    max_priority_fee_per_gas: Option<U256>,
    max_fee_per_gas: Option<U256>,
    gas_limit: U256,
    to: Option<H160>,
    value: U256,
    data: Vec<u8>,
    access_list: Option<AccessList>,
    v: U256,
    r: U256,
    s: U256,
}

fn decode_transaction(hex_tx: &str) -> Result<Transaction, Box<dyn std::error::Error>> {
    let raw_tx: Vec<u8> = hex::decode(utils::remove_0x_prefix(hex_tx))?;
    
    if raw_tx.is_empty() {
        return Err("Empty transaction data".into());
    }

    match raw_tx[0] {
        0x01 => decode_eip2930_transaction(&raw_tx[1..]),
        0x02 => decode_eip1559_transaction(&raw_tx[1..]),
        _ if raw_tx[0] > 0x7f => decode_legacy_transaction(raw_tx.as_slice()),
        _ => Err(format!("Unsupported transaction type: {:02x}", raw_tx[0]).into()),
    }
}

fn decode_legacy_transaction(raw_tx: &[u8]) -> Result<Transaction, Box<dyn std::error::Error>> {
    println!("Decoding legacy transaction");
    let rlp = Rlp::new(raw_tx);
    
    let item_count = rlp.item_count().map_err(|e| format!("Failed to get item count: {:?}", e))?;

    if item_count != 9 {
        return Err(format!("Expected 9 items for legacy transaction, got {}", item_count).into());
    }

    let to: Option<H160> = if rlp.at(3)?.is_empty() {
        None
    } else {
        Some(rlp.val_at(3).map_err(|e| format!("Failed to decode to: {:?}", e))?)
    };

    Ok(Transaction {
        transaction_type: TransactionType::Legacy,
        chain_id: None,
        nonce: rlp.val_at(0).map_err(|e| format!("Failed to decode nonce: {:?}", e))?,
        gas_price: Some(rlp.val_at(1).map_err(|e| format!("Failed to decode gas_price: {:?}", e))?),
        max_priority_fee_per_gas: None,
        max_fee_per_gas: None,
        gas_limit: rlp.val_at(2).map_err(|e| format!("Failed to decode gas_limit: {:?}", e))?,
        to,
        value: rlp.val_at(4).map_err(|e| format!("Failed to decode value: {:?}", e))?,
        data: rlp.val_at(5).map_err(|e| format!("Failed to decode data: {:?}", e))?,
        access_list: None,
        v: rlp.val_at(6).map_err(|e| format!("Failed to decode v: {:?}", e))?,
        r: rlp.val_at(7).map_err(|e| format!("Failed to decode r: {:?}", e))?,
        s: rlp.val_at(8).map_err(|e| format!("Failed to decode s: {:?}", e))?,
    })
}

fn decode_eip2930_transaction(raw_tx: &[u8]) -> Result<Transaction, Box<dyn std::error::Error>> {
    println!("Decoding EIP-2930 transaction");
    let rlp = Rlp::new(raw_tx);
    
    let item_count = rlp.item_count().map_err(|e| format!("Failed to get item count: {:?}", e))?;

    if item_count != 11 {
        return Err(format!("Expected 11 items for EIP-2930 transaction, got {}", item_count).into());
    }

    let to: Option<H160> = if rlp.at(4)?.is_empty() {
        None
    } else {
        Some(rlp.val_at(4).map_err(|e| format!("Failed to decode to: {:?}", e))?)
    };

    Ok(Transaction {
        transaction_type: TransactionType::EIP2930,
        chain_id: Some(rlp.val_at(0).map_err(|e| format!("Failed to decode chain_id: {:?}", e))?),
        nonce: rlp.val_at(1).map_err(|e| format!("Failed to decode nonce: {:?}", e))?,
        gas_price: Some(rlp.val_at(2).map_err(|e| format!("Failed to decode gas_price: {:?}", e))?),
        max_priority_fee_per_gas: None,
        max_fee_per_gas: None,
        gas_limit: rlp.val_at(3).map_err(|e| format!("Failed to decode gas_limit: {:?}", e))?,
        to,
        value: rlp.val_at(5).map_err(|e| format!("Failed to decode value: {:?}", e))?,
        data: rlp.val_at(6).map_err(|e| format!("Failed to decode data: {:?}", e))?,
        access_list: Some(rlp.val_at(7).map_err(|e| format!("Failed to decode access_list: {:?}", e))?),
        v: rlp.val_at(8).map_err(|e| format!("Failed to decode v: {:?}", e))?,
        r: rlp.val_at(9).map_err(|e| format!("Failed to decode r: {:?}", e))?,
        s: rlp.val_at(10).map_err(|e| format!("Failed to decode s: {:?}", e))?,
    })
}

fn decode_eip1559_transaction(raw_tx: &[u8]) -> Result<Transaction, Box<dyn std::error::Error>> {
    println!("Decoding EIP-1559 transaction");
    let rlp = Rlp::new(raw_tx);
    
    let item_count = rlp.item_count().map_err(|e| format!("Failed to get item count: {:?}", e))?;

    if item_count != 12 {
        return Err(format!("Expected 12 items for EIP-1559 transaction, got {}", item_count).into());
    }

    let to: Option<H160> = if rlp.at(5)?.is_empty() {
        None
    } else {
        Some(rlp.val_at(5).map_err(|e| format!("Failed to decode to: {:?}", e))?)
    };

    Ok(Transaction {
        transaction_type: TransactionType::EIP1559,
        chain_id: Some(rlp.val_at(0).map_err(|e| format!("Failed to decode chain_id: {:?}", e))?),
        nonce: rlp.val_at(1).map_err(|e| format!("Failed to decode nonce: {:?}", e))?,
        gas_price: None,
        max_priority_fee_per_gas: Some(rlp.val_at(2).map_err(|e| format!("Failed to decode max_priority_fee_per_gas: {:?}", e))?),
        max_fee_per_gas: Some(rlp.val_at(3).map_err(|e| format!("Failed to decode max_fee_per_gas: {:?}", e))?),
        gas_limit: rlp.val_at(4).map_err(|e| format!("Failed to decode gas_limit: {:?}", e))?,
        to,
        value: rlp.val_at(6).map_err(|e| format!("Failed to decode value: {:?}", e))?,
        data: rlp.val_at(7).map_err(|e| format!("Failed to decode data: {:?}", e))?,
        access_list: Some(rlp.val_at(8).map_err(|e| format!("Failed to decode access_list: {:?}", e))?),
        v: rlp.val_at(9).map_err(|e| format!("Failed to decode v: {:?}", e))?,
        r: rlp.val_at(10).map_err(|e| format!("Failed to decode r: {:?}", e))?,
        s: rlp.val_at(11).map_err(|e| format!("Failed to decode s: {:?}", e))?,
    })
}

pub fn txn_decoder(tx: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match decode_transaction(tx) {
        Ok(decoded) => {
            println!("Decoded Legacy Transaction:");
            println!("  Nonce: {:?}", decoded.nonce);
            println!("  Gas Price: {:?}", decoded.gas_price);
            println!("  Gas Limit: {:?}", decoded.gas_limit);
            println!("  To: {:?}", decoded.to.unwrap());
            println!("  Value: {:?}", decoded.value);
            println!("  Data length: {} bytes", decoded.data.len());
            println!("  V: {:?}", decoded.v);
            println!("  R: {:?}", decoded.r);
            println!("  S: {:?}", decoded.s);
            Ok(decoded.data)
        },
        Err(e) => Err(e),
    }
}