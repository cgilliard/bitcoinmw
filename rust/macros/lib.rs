extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro_derive(Dummy)]
pub fn derive_dummy(_input: TokenStream) -> TokenStream {
	// just hard code a simple text to implement partial equal for our type
	"impl PartialEq for MyStruct { fn eq(&self, _other: &MyStruct) -> bool { false } }"
		.parse()
		.expect("Failed to parse token stream")
}
