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

#[cfg(not(feature = "bench"))]
#[proc_macro_attribute]
pub fn weight(attr: TokenStream, item: TokenStream) -> TokenStream {
	let ItemFn { attrs, vis, sig, block } = syn::parse(item).unwrap();
	let weight: syn::Expr = syn::parse(attr).unwrap();
	(quote! {
		#(#attrs)*
		#vis #sig {
			::orml_weight_meter::using(#weight);
			#block
		}
	})
	.into()
}

#[cfg(feature = "bench")]
#[proc_macro_attribute]
pub fn weight(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let ItemFn { attrs, vis, sig, block } = syn::parse(item).unwrap();
	(quote! {
		#(#attrs)*
		#vis #sig {
			#[cfg(feature = "bench")]
			let identifier: ::sp_std::vec::Vec<u8> = ::orml_bencher::bencher::entering_method();
			let result = #block;
			#[cfg(feature = "bench")]
			::orml_bencher::bencher::leaving_method(&identifier);
			result
		}
	})
	.into()
}
