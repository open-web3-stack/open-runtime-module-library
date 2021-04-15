use proc_macro::TokenStream;
use quote::quote;
use syn::ItemFn;

mod method_injector;

#[proc_macro_attribute]
pub fn start_with(attr: TokenStream, item: TokenStream) -> TokenStream {
	let base_weight: syn::Expr = syn::parse(attr).unwrap();
	let ItemFn { attrs, vis, sig, block } = syn::parse(item).unwrap();
	(quote! {
		#(#attrs)*
		#vis #sig {
			::orml_weight_meter::start_with(#base_weight);
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

#[proc_macro_attribute]
pub fn method_benchmarks(_attr: TokenStream, input: TokenStream) -> TokenStream {
	#[cfg(feature = "runtime-benchmarks")]
	return method_injector::inject_methods(input);
	#[cfg(not(feature = "runtime-benchmarks"))]
	return input;
}
