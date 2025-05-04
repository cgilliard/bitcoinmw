#![no_std]

#[macro_use]
extern crate base;
use base::prelude::*;
use core::str::FromStr;

extern crate proc_macro;
use proc_macro::TokenStream;

fn try_derive(_input: TokenStream) -> Result<TokenStream> {
	let _e: Result<()> = err!(IllegalState);
	let ret = TokenStream::from_str(
		"impl PartialEq for MyStruct { fn eq(&self, _other: &MyStruct) -> bool { false } }",
	)
	.map_err(|_e| ParseError)?;
	Ok(ret)
}

#[proc_macro_derive(Dummy)]
pub fn derive_dummy(input: TokenStream) -> TokenStream {
	try_derive(input).expect("Could not parse tokenstream!")
}
