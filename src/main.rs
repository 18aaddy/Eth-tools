use ethereum_types::{H160, U256, H256};
use rlp::{Rlp, RlpStream, Decodable};
use secp256k1::SecretKey;
use sha3::{Digest, Keccak256};
use std::str::FromStr;

#[derive(Debug)]
enum TransactionType {
    Legacy,
    EIP2930,
    EIP1559,
}

// impl Decodable for Vec<(H160, Vec<H256>)> {
//     fn decode(rlp: &Rlp) -> Result<Self, rlp::DecoderError> {
//         let mut result = Vec::new();
//         for i in 0..rlp.item_count()? {
//             let entry: (H160, Vec<H256>) = rlp.val_at(i)?;
//             result.push(entry);
//         }
//         Ok(result)
//     }
// }

#[derive(Debug)]
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
    // access_list: Option<Vec<(H160, Vec<H256>)>>,
    v: U256,
    r: U256,
    s: U256,
}

fn decode_transaction(raw_tx: &[u8]) -> Result<Transaction, Box<dyn std::error::Error>> {
    if raw_tx[0] > 0x7f {
        decode_legacy_transaction(&raw_tx)
    } else {
        let tx_type = raw_tx[0];
        match tx_type {
            0x01 => decode_eip2930_transaction(&raw_tx[1..]),
            0x02 => decode_eip1559_transaction(&raw_tx[1..]),
            _ => Err("Unsupported transaction type".into()),
        }
    }
}

fn decode_legacy_transaction(raw_tx: &[u8]) -> Result<Transaction, Box<dyn std::error::Error>> {
    let rlp = Rlp::new(raw_tx);
    Ok(Transaction {
        transaction_type: TransactionType::Legacy,
        chain_id: None, // Chain ID is derived from v in legacy transactions
        nonce: rlp.val_at(0)?,
        gas_price: Some(rlp.val_at(1)?),
        max_priority_fee_per_gas: None,
        max_fee_per_gas: None,
        gas_limit: rlp.val_at(2)?,
        to: rlp.val_at(3)?,
        value: rlp.val_at(4)?,
        data: rlp.val_at(5)?,
        // access_list: None,
        v: rlp.val_at(6)?,
        r: rlp.val_at(7)?,
        s: rlp.val_at(8)?,
    })
}

fn decode_eip2930_transaction(raw_tx: &[u8]) -> Result<Transaction, Box<dyn std::error::Error>> {
    let rlp = Rlp::new(raw_tx);
    Ok(Transaction {
        transaction_type: TransactionType::EIP2930,
        chain_id: Some(rlp.val_at(0)?),
        nonce: rlp.val_at(1)?,
        gas_price: Some(rlp.val_at(2)?),
        max_priority_fee_per_gas: None,
        max_fee_per_gas: None,
        gas_limit: rlp.val_at(3)?,
        to: rlp.val_at(4)?,
        value: rlp.val_at(5)?,
        data: rlp.val_at(6)?,
        // access_list: Some(rlp.val_at(7)?),
        v: rlp.val_at(8)?,
        r: rlp.val_at(9)?,
        s: rlp.val_at(10)?,
    })
}

fn decode_eip1559_transaction(raw_tx: &[u8]) -> Result<Transaction, Box<dyn std::error::Error>> {
    let rlp = Rlp::new(raw_tx);
    Ok(Transaction {
        transaction_type: TransactionType::EIP1559,
        chain_id: Some(rlp.val_at(0)?),
        nonce: rlp.val_at(1)?,
        gas_price: None,
        max_priority_fee_per_gas: Some(rlp.val_at(2)?),
        max_fee_per_gas: Some(rlp.val_at(3)?),
        gas_limit: rlp.val_at(4)?,
        to: rlp.val_at(5)?,
        value: rlp.val_at(6)?,
        data: rlp.val_at(7)?,
        // access_list: Some(rlp.val_at(8)?),
        v: rlp.val_at(9)?,
        r: rlp.val_at(10)?,
        s: rlp.val_at(11)?,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example usage
    let legacy_tx = hex::decode("f86c098504a817c800825208943535353535353535353535353535353535353535880de0b6b3a76400008025a028ef61340bd939bc2195fe537567866003e1a15d3c71ff63e1590620aa636276a067cbe9d8997f761aecb703304b3800ccf555c9f3dc64214b297fb1966a3b6d83")?;
    let decoded_legacy = decode_transaction(&legacy_tx)?;
    println!("Decoded Legacy Transaction: {:?}", decoded_legacy);

    // You would add similar examples for EIP-2930 and EIP-1559 transactions here

    Ok(())
}