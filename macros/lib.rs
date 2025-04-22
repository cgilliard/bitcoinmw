extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro_derive(Dummy)]
pub fn derive_my_clone(_input: TokenStream) -> TokenStream {
	TokenStream::new()
}
