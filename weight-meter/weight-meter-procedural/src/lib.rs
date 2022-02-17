use proc_macro::TokenStream;
use quote::quote;
use syn::{parse, Expr, ItemFn};

#[proc_macro_attribute]
pub fn start(attr: TokenStream, item: TokenStream) -> TokenStream {
	let weight: Expr = if attr.is_empty() {
		parse((quote! { 0 }).into()).unwrap()
	} else {
		parse(attr).unwrap()
	};
	let ItemFn { attrs, vis, sig, block } = parse(item).unwrap();
	(quote! {
		#(#attrs)*
		#[cfg_attr(feature = "bench", ::orml_bencher::benchmarkable)]
		#vis #sig {
			::orml_weight_meter::start(#weight);
			let result = #block;
			::orml_weight_meter::finish();
			result
		}
	})
	.into()
}

#[proc_macro_attribute]
pub fn weight(attr: TokenStream, item: TokenStream) -> TokenStream {
	let weight: Expr = parse(attr).unwrap();
	let ItemFn { attrs, vis, sig, block } = parse(item).unwrap();
	(quote! {
		#(#attrs)*
		#[cfg_attr(feature = "bench", ::orml_bencher::benchmarkable)]
		#vis #sig {
			::orml_weight_meter::using(#weight);
			#block
		}
	})
	.into()
}
