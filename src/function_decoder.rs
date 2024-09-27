// use reqwest::Error as ReqwestError;
use serde::Deserialize;
use anyhow::{Error, Result};
use futures::future::BoxFuture;
use futures::FutureExt; // For `.boxed()`
use crate::utils;
use num_bigint::BigUint;
use num_traits::Num;

#[derive(Deserialize, Debug)]
struct SignatureResponse {
    count: usize,
    results: Vec<FunctionSignature>,
}

#[derive(Deserialize, Debug)]
struct FunctionSignature {
    text_signature: String,
}

pub async fn final_result_from_calldata(mut call_data: &str) -> Result<(), Error> {
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

    let extracted_params = extract_params(&function_signature); // This function is assumed to be defined elsewhere
    let new_params: Vec<&str> = extracted_params.iter().map(|s| s.as_str()).collect();
    let str_params: &[&str] = &new_params;
    let params = decode_calldata(call_data, str_params);
    println!("Function signature: {}", function_signature);
    println!("Parameters: {:?}", params);
    // call_data = &call_data[index..]; // Remove the decoded part from calldata
    // final_result_from_calldata(call_data).await?;
    Ok(())
}


fn get_selector_from_call_data(call_data: &str) -> Result<String, Error> {
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

fn extract_params(function_signature: &str) -> Vec<String> {
    // Find the opening and closing parentheses
    if let Some(start) = function_signature.find('(') {
        if let Some(end) = function_signature.find(')') {
            // Extract the part between the parentheses
            let params_str = &function_signature[start + 1..end];

            // Split by commas and collect into a vector of parameter types
            return params_str
                .split(',')
                .map(|param| param.trim().to_string()) // Trim whitespace and collect params
                .collect();
        }
    }
    
    // If no parameters found, return an empty vector
    vec![]
}

fn extract_params_from_calldata(extracted_params: &Vec<String>, call_data: &str) -> (Result<Vec<String>, Error>, usize) {
    let mut params: Vec<String> = Vec::new();
    let call_data = utils::remove_0x_prefix(call_data);
    let mut index = 0; // Track the position in calldata

    for param_type in extracted_params {
        match param_type.as_str() {
            "bool" => {
                params.push(format!("{}", &call_data[index..index + 2])); // 1 byte for bool
                index += 2;
            }
            // Integers
            "uint256" | "uint" | "int" | "int256" => {
                params.push(format!("{}", &call_data[index..index + 64])); // 32 bytes (256 bits)
                index += 64;
            }
            "uint128" | "int128" => {
                params.push(format!("{}", &call_data[index..index + 32])); // 16 bytes (128 bits)
                index += 32;
            }
            "uint64" | "int64" => {
                params.push(format!("{}", &call_data[index..index + 16])); // 8 bytes (64 bits)
                index += 16;
            }
            "uint32" | "int32" => {
                params.push(format!("{}", &call_data[index..index + 8])); // 4 bytes (32 bits)
                index += 8;
            }
            "uint16" | "int16" => {
                params.push(format!("{}", &call_data[index..index + 4])); // 2 bytes (16 bits)
                index += 4;
            }
            "uint8" | "int8" => {
                params.push(format!("{}", &call_data[index..index + 2])); // 1 byte (8 bits)
                index += 2;
            }
            // Address
            "address" => {
                params.push(format!("{}", &call_data[index..index + 40])); // 20 bytes (40 hex characters)
                index += 40;
            }
            // For dynamic types
            "string" => {
                // Read length (4 bytes)
                let length_hex = &call_data[index..index + 8];
                let length = match u64::from_str_radix(length_hex, 16) {
                    Ok(l) => l,
                    Err(e) => return (Err(Error::msg(e.to_string())), index),
                }; // Convert hex to length
                index += 8; // Move index forward
                
                // Read actual string bytes (length * 2 for hex)
                let string_bytes = &call_data[index..index + (length * 2) as usize];
                params.push(format!("{}", string_bytes)); // Add string bytes
                index += (length * 2) as usize; // Move index forward
            }
            "bytes" => {
                // Read length (4 bytes)
                let length_hex = &call_data[index..index + 8];
                let length = match u64::from_str_radix(length_hex, 16) {
                    Ok(l) => l,
                    Err(e) => return (Err(Error::msg(e.to_string())), index),
                }; // Convert hex to length
                index += 8; // Move index forward
                
                // Read actual bytes (length * 2 for hex)
                let byte_array = &call_data[index..index + (length * 2) as usize];
                params.push(format!("{}", byte_array)); // Add bytes
                index += (length * 2) as usize; // Move index forward
            }
            "address[]" => {
                // Read length (4 bytes)
                let array_offset = usize::from_str_radix(&call_data[index..index + 64], 16).unwrap();
                index += array_offset * 2; // Move index forward
                let array_length = usize::from_str_radix(&call_data[index..index + 64], 16).unwrap();
                index += 64; // Move index forward
                // Read actual bytes (length * 2 for hex)
                let array_data = &call_data[index..index + (array_length * 20 * 2) as usize];
                params.push(format!("{}", array_data)); // Add array data
                index += (array_length * 2) as usize; // Move index forward
            }
            "bytes[]" => {
                // Read length (4 bytes)
                let array_offset = usize::from_str_radix(&call_data[index..index + 64], 16).unwrap();
                index += array_offset * 2; // Move index forward
                let array_length = usize::from_str_radix(&call_data[index..index + 64], 16).unwrap();
                index += 64; // Move index forward
                // Read actual bytes (length * 2 for hex)
                let array_data = &call_data[index..index + (array_length * 2 * 2) as usize];
                params.push(format!("{}", array_data)); // Add array data
                index += (array_length * 2) as usize; // Move index forward
            }
            _ => {
                println!("Unknown type: {}", param_type);
            }
        }
    }

    (Ok(params), index)
}

fn decode_calldata(calldata: &str, types: &[&str]) -> Vec<String> {
    let calldata = utils::remove_0x_prefix(calldata);
    let mut params: Vec<String> = Vec::new();
    let mut dynamic_offsets: Vec<usize> = Vec::new();
    let mut offset = 8; // Skip the first 4 bytes for the function selector

    // First pass: decode static types and collect dynamic offsets
    for &typ in types {
        match typ {
            "address" => {
                let param = &calldata[offset + 24..offset + 64]; // last 20 bytes for address
                params.push(format!("0x{}", param));
            }
            "uint256" | "int256" | "bool" => {
                let param = &calldata[offset..offset + 64]; // 32-byte value
                params.push(decode_integer_or_bool(param, typ));
            }
            // For dynamic types, we store their offset for later decoding
            "string" | "bytes" | "address[]" | "uint256[]" | "string[]" => {
                let dynamic_offset = usize::from_str_radix(&calldata[offset..offset + 64], 16).unwrap() * 2;
                dynamic_offsets.push(dynamic_offset);
                params.push(format!("Dynamic offset: {}", dynamic_offset));
            }
            _ => {
                println!("Unsupported type: {}", typ);
            }
        }
        offset += 64; // Move to the next parameter (32 bytes per parameter)
    }

    // Second pass: decode dynamic types
    for (i, &typ) in types.iter().enumerate() {
        if let Some(dynamic_offset) = dynamic_offsets.get(i) {
            match typ {
                "string" => {
                    let decoded_string = decode_string(calldata, *dynamic_offset);
                    params[i] = decoded_string;
                }
                "bytes" => {
                    let decoded_bytes = decode_bytes(calldata, *dynamic_offset);
                    params[i] = decoded_bytes;
                }
                "address[]" => {
                    let decoded_array = decode_address_array(calldata, *dynamic_offset);
                    params[i] = format!("{:?}", decoded_array);
                }
                "uint256[]" => {
                    let decoded_array = decode_uint_array(calldata, *dynamic_offset);
                    params[i] = format!("{:?}", decoded_array);
                }
                "string[]" => {
                    let decoded_array = decode_string_array(calldata, *dynamic_offset);
                    params[i] = format!("{:?}", decoded_array);
                }
                _ => {}
            }
        }
    }

    params
}

fn decode_integer_or_bool(data: &str, typ: &str) -> String {
    match typ {
        "bool" => {
            if &data[63..64] == "1" {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        _ => BigUint::from_str_radix(data, 16).unwrap().to_string(), // Handle integers like uint256
    }
}

fn decode_string(calldata: &str, offset: usize) -> String {
    let string_length = usize::from_str_radix(&calldata[offset..offset + 64], 16).unwrap();
    let string_data = &calldata[offset + 64..offset + 64 + string_length * 2];
    hex_to_utf8(string_data)
}

fn decode_bytes(calldata: &str, offset: usize) -> String {
    let bytes_length = usize::from_str_radix(&calldata[offset..offset + 64], 16).unwrap();
    let bytes_data = &calldata[offset + 64..offset + 64 + bytes_length * 2];
    format!("0x{}", bytes_data)
}

fn decode_address_array(calldata: &str, offset: usize) -> Vec<String> {
    let array_length = usize::from_str_radix(&calldata[offset..offset + 64], 16).unwrap();
    let mut addresses: Vec<String> = Vec::new();
    let mut current_offset = offset + 64;

    for _ in 0..array_length {
        let address = &calldata[current_offset + 24..current_offset + 64]; // Last 20 bytes
        addresses.push(format!("0x{}", address));
        current_offset += 64;
    }
    addresses
}

fn decode_uint_array(calldata: &str, offset: usize) -> Vec<String> {
    let array_length = usize::from_str_radix(&calldata[offset..offset + 64], 16).unwrap();
    let mut uints: Vec<String> = Vec::new();
    let mut current_offset = offset + 64;

    for _ in 0..array_length {
        let value = &calldata[current_offset..current_offset + 64];
        uints.push(u64::from_str_radix(value, 16).unwrap().to_string());
        current_offset += 64;
    }
    uints
}

fn decode_string_array(calldata: &str, offset: usize) -> Vec<String> {
    let array_length = usize::from_str_radix(&calldata[offset..offset + 64], 16).unwrap();
    let mut strings: Vec<String> = Vec::new();
    let mut current_offset = offset + 64;

    for _ in 0..array_length {
        let string_offset = usize::from_str_radix(&calldata[current_offset..current_offset + 64], 16).unwrap() * 2;
        let string_length = usize::from_str_radix(&calldata[string_offset..string_offset + 64], 16).unwrap();
        let string_data = &calldata[string_offset + 64..string_offset + 64 + string_length * 2];
        let decoded_string = hex_to_utf8(string_data);
        strings.push(decoded_string);
        current_offset += 64;
    }
    strings
}

fn hex_to_utf8(hex: &str) -> String {
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect::<Vec<u8>>();
    String::from_utf8(bytes).unwrap()
}