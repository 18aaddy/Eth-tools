use serde::Deserialize;
use anyhow::{Error, Result};
use crate::utils;
use ethabi::Token;
use ethabi::ParamType;
use tiny_keccak::{Hasher, Keccak};
use hex;
use regex::Regex;

#[derive(Deserialize, Debug)]
struct SignatureResponse {
    count: usize,
    results: Vec<FunctionSignature>,
}

#[derive(Deserialize, Debug)]
struct FunctionSignature {
    text_signature: String,
}

pub async fn final_result_from_calldata(call_data: &str) -> Result<(), Error> {
    // Check if the call_data is empty or too short
    if call_data.is_empty() || call_data.len() < 10 {
        println!("Invalid calldata, nothing to decode.");
        return Err(Error::msg("No valid function in calldata."));
    }

    let function_selector = match get_selector_from_call_data(call_data) {
        Ok(selector) => selector,
        Err(e) => {
            println!("Error extracting function selector: {}", e);
            return Err(e);
        }
    };

    let function_signature = match get_function_signature(&function_selector).await {
        Ok(signature) => signature,
        Err(e) => {
            println!("Error fetching function signature: {}", e);
            return Err(e);
        }
    };

    if function_signature.is_empty() {
        println!("No function signature found");
        return Err(Error::msg("No function signature found"));
    }

    let err = decode_without_abi(function_signature.as_str(), call_data).err(); 
    if let Some(err) = err {
        println!("Error decoding calldata: {:?}", err);
        return Err(Error::msg("Error decoding calldata"));
    }

    Ok(())
}


pub fn get_selector_from_call_data(call_data: &str) -> Result<String, Error> {
    let call_data = utils::remove_0x_prefix(call_data);
    if call_data.len() < 8 {
        return Err(Error::msg("Call data is too short"));
    }
    let function_selector = &call_data[0..8];
    Ok(format!("0x{}", function_selector))
}

pub async fn get_function_signature(function_selector: &str) -> Result<String, Error> {
    let url = format!("https://www.4byte.directory/api/v1/signatures/?hex_signature={}", function_selector);
    let response = reqwest::get(&url).await?
        .json::<SignatureResponse>()
        .await?;

    if response.count == 0 {
        println!("No function signature found");
        return Err(Error::msg("No function signature found"));
    }

    Ok(response.results[0].text_signature.clone())
}

pub struct CalldataDecoder {
    function_signature: String,
    param_types: Vec<ParamType>,
}

impl CalldataDecoder {
    pub fn new(function_signature: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (param_types, _) = Self::parse_signature(function_signature)?;
        Ok(Self {
            function_signature: function_signature.to_string(),
            param_types,
        })
    }

    pub fn decode_calldata(&self, calldata: &str) -> Result<Vec<Token>, Box<dyn std::error::Error>> {
        let calldata = hex::decode(calldata.strip_prefix("0x").unwrap_or(calldata))?;
        
        // Verify function selector
        let expected_selector = self.compute_function_selector(&self.function_signature);
        if calldata[..4] != expected_selector {
            return Err("Function selector mismatch".into());
        }

        // Decode parameters
        ethabi::decode(&self.param_types, &calldata[4..]).map_err(|e| e.into())
    }

    fn parse_signature(signature: &str) -> Result<(Vec<ParamType>, Option<ParamType>), Box<dyn std::error::Error>> {
        let re = Regex::new(r"(\w+)\((.*)\)(\s*->\s*(.+))?").unwrap();
        let caps = re.captures(signature).ok_or("Invalid function signature")?;

        let params_str = caps.get(2).map_or("", |m| m.as_str());
        let return_type_str = caps.get(4).map(|m| m.as_str());

        let param_types = params_str
            .split(',')
            .filter(|s| !s.is_empty())
            .map(Self::parse_type)
            .collect::<Result<Vec<_>, _>>()?;

        let return_type = return_type_str.map(Self::parse_type).transpose()?;

        Ok((param_types, return_type))
    }

    fn parse_type(typ: &str) -> Result<ParamType, Box<dyn std::error::Error>> {
        let typ = typ.trim();

        // Check for array types first
        if typ.ends_with("[]") {
            let inner = Self::parse_type(&typ[..typ.len() - 2])?;
            return Ok(ParamType::Array(Box::new(inner)));
        }

        // Check for fixed-size array
        if let Some(captures) = Regex::new(r"^(.+)\[(\d+)\]$")?.captures(typ) {
            let inner = Self::parse_type(captures.get(1).unwrap().as_str())?;
            let size = captures.get(2).unwrap().as_str().parse::<usize>()?;
            return Ok(ParamType::FixedArray(Box::new(inner), size));
        }

        // Parse other types
        match typ {
            "address" => Ok(ParamType::Address),
            "bool" => Ok(ParamType::Bool),
            "string" => Ok(ParamType::String),
            "bytes" => Ok(ParamType::Bytes),

            // Unsigned integers
            t if t.starts_with("uint") => {
                let size = t[4..].parse::<usize>().map_err(|_| "Invalid uint size")?;
                if size % 8 == 0 && size <= 256 {
                    Ok(ParamType::Uint(size))
                } else {
                    Err("Invalid uint size".into())
                }
            },

            // Signed integers
            t if t.starts_with("int") => {
                let size = t[3..].parse::<usize>().map_err(|_| "Invalid int size")?;
                if size % 8 == 0 && size <= 256 {
                    Ok(ParamType::Int(size))
                } else {
                    Err("Invalid int size".into())
                }
            },

            // Fixed-size bytes
            t if t.starts_with("bytes") => {
                if t == "bytes" {
                    Ok(ParamType::Bytes)
                } else {
                    let size = t[5..].parse::<usize>().map_err(|_| "Invalid bytes size")?;
                    if size > 0 && size <= 32 {
                        Ok(ParamType::FixedBytes(size))
                    } else {
                        Err("Invalid fixed bytes size".into())
                    }
                }
            },

            // Tuples (e.g., "(uint256,address)")
            t if t.starts_with("(") && t.ends_with(")") => {
                let inner = &t[1..t.len() - 1];
                let inner_types = Self::split_tuple_types(inner)?
                    .into_iter()
                    .map(|t| Self::parse_type(&t))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ParamType::Tuple(inner_types))
            },

            // Add support for fixed point numbers if needed
            // "fixed" | "ufixed" => {...}

            _ => Err(format!("Unsupported type: {}", typ).into()),
        }
    }

    fn split_tuple_types(tuple_str: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut paren_count = 0;

        for ch in tuple_str.chars() {
            match ch {
                '(' => {
                    paren_count += 1;
                    current.push(ch);
                },
                ')' => {
                    paren_count -= 1;
                    current.push(ch);
                },
                ',' if paren_count == 0 => {
                    if !current.is_empty() {
                        result.push(current.trim().to_string());
                        current.clear();
                    }
                },
                _ => current.push(ch),
            }
        }

        if !current.is_empty() {
            result.push(current.trim().to_string());
        }

        if paren_count != 0 {
            return Err("Mismatched parentheses in tuple type".into());
        }

        Ok(result)
    }

    fn compute_function_selector(&self, signature: &str) -> [u8; 4] {
        let mut keccak = Keccak::v256();
        keccak.update(signature.as_bytes());
        let mut hash = [0u8; 32];
        keccak.finalize(&mut hash);
        [hash[0], hash[1], hash[2], hash[3]]
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

// Example usage
fn decode_without_abi(function_signature: &str, calldata: &str) -> Result<(), Box<dyn std::error::Error>> {
    let decoder = CalldataDecoder::new(function_signature)?;

    let decoded_params = decoder.decode_calldata(calldata)?;

    println!("Decoded parameters:");
    for (i, param) in decoded_params.iter().enumerate() {
        println!("  Parameter {}: {}", i, token_to_string(param));
    }

    Ok(())
}