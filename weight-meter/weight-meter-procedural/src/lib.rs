use proc_macro::TokenStream;
use quote::quote;
use syn::ItemFn;

#[proc_macro_attribute]
pub fn start(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let ItemFn { attrs, vis, sig, block } = syn::parse(item).unwrap();
	(quote! {
		#(#attrs)*
		#vis #sig {
			::orml_weight_meter::start();
			let result = #block;
			::orml_weight_meter::finish();
			result
		}
	})
	.into()
}

#[proc_macro_attribute]
pub fn weight(attr: TokenStream, item: TokenStream) -> TokenStream {
	let weight: syn::Expr = syn::parse(attr).unwrap();
	let ItemFn { attrs, vis, sig, block } = syn::parse(item).unwrap();
	(quote! {
		#(#attrs)*
		#vis #sig {
			::orml_weight_meter::using(#weight);
			#block
		}
	})
	.into()
}
