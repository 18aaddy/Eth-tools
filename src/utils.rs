pub fn remove_0x_prefix(hex_string: &str) -> &str {
    if hex_string.starts_with("0x") || hex_string.starts_with("0X") {
        &hex_string[2..]
    } else {
        hex_string
    }
}