/// Parse rod control
///
/// # Errors
/// If user provided wrong value
pub fn rod_position_parser(s: &str) -> Result<u16, String> {
    let val = s
        .parse()
        .map_err(|_| format!("`{s}` is not a valid number"))?;

    let min = 1;
    let max = 9;
    if (min..=max).contains(&val) {
        Ok(val)
    } else {
        Err(format!("Value must be between {min} and {max}, got {val}"))
    }
}
