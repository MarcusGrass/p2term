use core::fmt::Formatter;
use std::error::Error;

pub struct ErrFmt<'a>(&'a dyn core::error::Error);

#[inline]
pub fn unpack(e: &dyn Error) -> ErrFmt<'_> {
    ErrFmt(e)
}

impl core::fmt::Display for ErrFmt<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))?;
        let mut src = self.0.source();
        while let Some(e) = src {
            f.write_fmt(format_args!(" -> {e}"))?;
            src = e.source();
        }
        Ok(())
    }
}
