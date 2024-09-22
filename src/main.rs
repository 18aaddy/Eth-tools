use ethereum_types::{H160, U256, H256};
use rlp::{Rlp, Decodable, DecoderError};

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

fn decode_transaction(raw_tx: &[u8]) -> Result<Transaction, Box<dyn std::error::Error>> {
    println!("Raw transaction: {:?}", hex::encode(raw_tx));
    
    if raw_tx.is_empty() {
        return Err("Empty transaction data".into());
    }

    match raw_tx[0] {
        0x01 => decode_eip2930_transaction(&raw_tx[1..]),
        0x02 => decode_eip1559_transaction(&raw_tx[1..]),
        _ if raw_tx[0] > 0x7f => decode_legacy_transaction(raw_tx),
        _ => Err(format!("Unsupported transaction type: {:02x}", raw_tx[0]).into()),
    }
}

fn decode_legacy_transaction(raw_tx: &[u8]) -> Result<Transaction, Box<dyn std::error::Error>> {
    println!("Decoding legacy transaction");
    let rlp = Rlp::new(raw_tx);
    
    let item_count = rlp.item_count().map_err(|e| format!("Failed to get item count: {:?}", e))?;
    println!("Legacy transaction item count: {}", item_count);

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
    println!("EIP-2930 transaction item count: {}", item_count);

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
    println!("EIP-1559 transaction item count: {}", item_count);

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let legacy_tx = hex::decode("f86b808509502f900082520894423163e58aabec5daa3dd1130b759d24bef0f6ea8711c37937e080008026a07e9dfb6cb349d3bce51f01676ee5eff957321d34d0c2a293ee23440150092b73a06cd7c96729fdf3daf75ea12285d9b9178d44695e6f0e7138b69797198c4f184f")?;
    let eip1559_tx = hex::decode("02f8b70120830f4240850235c20b228305e81894a436c9048d4927ff69943278aae0e426f9f68755870a3e2aefcf5f00b8442c65169e00000000000000000000000000000000000000000000000000000000000013880000000000000000000000000000000000000000000000000000000000000001c001a03143ad69303279831b4716f595a33be890167259f6c212f1aa70e0e68363ff52a0185006794dc10cf1cb9e3aa4ad5ecf48ef1e0456ef14ec1f2024eb36b5b1812d")?;
    
    println!("Transaction hex: {}", hex::encode(&legacy_tx));
    
    match decode_transaction(&legacy_tx) {
        Ok(decoded) => {
            println!("Decoded Legacy Transaction:");
            println!("  Nonce: {:?}", decoded.nonce);
            println!("  Gas Price: {:?}", decoded.gas_price);
            println!("  Gas Limit: {:?}", decoded.gas_limit);
            println!("  To: {:?}", decoded.to);
            println!("  Value: {:?}", decoded.value);
            println!("  Data length: {} bytes", decoded.data.len());
            println!("  V: {:?}", decoded.v);
            println!("  R: {:?}", decoded.r);
            println!("  S: {:?}", decoded.s);
        },
        Err(e) => println!("Error decoding transaction: {}", e),
    }

    println!("Transaction hex: {}", hex::encode(&eip1559_tx));
    
    match decode_transaction(&eip1559_tx) {
        Ok(decoded) => {
            println!("Decoded Legacy Transaction:");
            println!("  Nonce: {:?}", decoded.nonce);
            println!("  Gas Price: {:?}", decoded.gas_price);
            println!("  Gas Limit: {:?}", decoded.gas_limit);
            println!("  To: {:?}", decoded.to);
            println!("  Value: {:?}", decoded.value);
            println!("  Data length: {:?} bytes", decoded.data);
            println!("  V: {:?}", decoded.v);
            println!("  R: {:?}", decoded.r);
            println!("  S: {:?}", decoded.s);
        },
        Err(e) => println!("Error decoding transaction: {}", e),
    }

    Ok(())
}