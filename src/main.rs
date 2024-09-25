use reqwest::Error;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct SignatureResponse {
    count: usize,
    results: Vec<FunctionSignature>,
}

#[derive(Deserialize, Debug)]
struct FunctionSignature {
    id: u64,
    created_at: String,
    text_signature: String,
    hex_signature: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let function_selector = "0x07883703";

    // Construct the API URL
    let url = format!("https://www.4byte.directory/api/v1/signatures/?hex_signature={}", function_selector);

    // Send the GET request
    let response = reqwest::get(&url).await?
        .json::<SignatureResponse>()
        .await?;

    // Print the retrieved signatures
    for signature in response.results {
        println!("Function Signature: {}", signature.text_signature);
    }

    Ok(())
}