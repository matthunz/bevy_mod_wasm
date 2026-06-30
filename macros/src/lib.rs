use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let attrs = &input.attrs;
    let block = &input.block;

    quote! {
        ::bevy_mod_wasm::guest_exports!();

        #(#attrs)*
        #[unsafe(no_mangle)]
        pub extern "C" fn main() #block
    }
    .into()
}
