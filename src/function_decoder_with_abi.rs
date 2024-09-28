use ethabi::{Contract, Token};
use hex;

pub struct CalldataDecoder {
    contract: Contract,
}

impl CalldataDecoder {
    pub fn new(abi_json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contract = Contract::load(abi_json.as_bytes())?;
        Ok(Self { contract })
    }

    pub fn decode_calldata(&self, calldata: &str) -> Result<(String, Vec<Token>), Box<dyn std::error::Error>> {
        let calldata = hex::decode(calldata.strip_prefix("0x").unwrap_or(calldata))?;
        
        // Extract function selector (first 4 bytes)
        let function_selector = &calldata[0..4];

        // Find the matching function in the ABI
        let function = self.contract.functions().find(|f| {
            f.short_signature() == function_selector
        }).ok_or("Function not found in ABI")?;

        // Decode the parameters
        let decoded = function.decode_input(&calldata[4..])?;

        Ok((function.name.clone(), decoded))
    }
}

// Helper function to convert ethabi::Token to a more readable format
pub fn token_to_string(token: &Token) -> String {
    match token {
        Token::Address(address) => format!("Address: 0x{:x}", address),
        Token::Uint(uint) => format!("Uint: {}", uint),
        Token::Int(int) => format!("Int: {}", int),
        Token::Bool(b) => format!("Bool: {}", b),
        Token::String(s) => format!("String: {}", s),
        Token::Bytes(bytes) => format!("Bytes: 0x{}", hex::encode(bytes)),
        Token::FixedBytes(bytes) => format!("FixedBytes: 0x{}", hex::encode(bytes)),
        Token::Array(tokens) => format!("Array: [{}]", tokens.iter().map(token_to_string).collect::<Vec<_>>().join(", ")),
        Token::FixedArray(tokens) => format!("FixedArray: [{}]", tokens.iter().map(token_to_string).collect::<Vec<_>>().join(", ")),
        Token::Tuple(tokens) => format!("Tuple: ({})", tokens.iter().map(token_to_string).collect::<Vec<_>>().join(", ")),
    }
}