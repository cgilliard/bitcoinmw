use core::result::Result as CoreResult;
use std::error::Error;

pub type Result<T> = CoreResult<T, Error>;
