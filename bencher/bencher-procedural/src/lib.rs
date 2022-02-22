use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn benchmarkable(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let syn::ItemFn { attrs, vis, sig, block } = syn::parse(item).unwrap();
	(quote::quote! {
		#(#attrs)*
		#vis #sig {
			#[cfg(not(feature = "std"))]
			::orml_bencher::bench::before_block();
			let result = #block;
			#[cfg(not(feature = "std"))]
			::orml_bencher::bench::after_block();
			result
		}
	})
	.into()
}
