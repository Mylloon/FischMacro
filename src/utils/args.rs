/// Parse rod control
///
/// # Errors
/// If user provided wrong value
pub fn rod_control_parser(s: &str) -> Result<f32, String> {
    let val: f32 = s
        .parse()
        .map_err(|_| format!("`{s}` is not a valid number"))?;

    let min = 0.;
    let max = 0.7;
    if (min..=max).contains(&val) {
        Ok(val)
    } else {
        Err(format!("Value must be between {min} and {max}, got {val}"))
    }
}
