pub trait BadCast {
    type Output;

    /// Do a simple cast `T` as `U`.
    ///
    /// This can lead to truncation or precision loss.
    #[must_use]
    fn bad_cast(self) -> Self::Output;
}

#[allow(clippy::cast_possible_truncation)]
impl BadCast for f32 {
    type Output = i32;

    fn bad_cast(self) -> i32 {
        self as i32
    }
}

#[allow(clippy::cast_precision_loss)]
impl BadCast for i32 {
    type Output = f32;

    fn bad_cast(self) -> f32 {
        self as f32
    }
}
