#![allow(unused_imports)]

use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{Attribute, FnArg, Ident, ImplItem, ImplItemMethod, Item, ItemFn, ItemImpl, ItemMod, Pat};

#[proc_macro_attribute]
pub fn start_with(attr: TokenStream, item: TokenStream) -> TokenStream {
	let base_weight: syn::Expr = syn::parse(attr).unwrap();
	let ItemFn { attrs, vis, sig, block } = syn::parse(item).unwrap();
	(quote! {
		#(#attrs)*
		#vis #sig {
			weight_meter::start_with(#base_weight);
			let result = #block;
			weight_meter::end();
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
			weight_meter::using(#weight);
			#block
		}
	})
	.into()
}

#[proc_macro_attribute]
pub fn method_benchmarks(_attr: TokenStream, input: TokenStream) -> TokenStream {
	#[cfg(feature = "runtime-benchmarks")] // Inject methods if we're benchmarking
	{
		let mut methods: Vec<ImplItem> = vec![];

		let mut item: ItemMod = syn::parse(input).unwrap();
		let (brace, content) = item.content.clone().unwrap();

		let whitelist = find_methods(&content);

		// Generate callable methods dynamically
		content.iter().for_each(|item| {
			if let Item::Impl(ItemImpl { items, .. }) = item {
				items.iter().for_each(|item_impl| {
					if let ImplItem::Method(ImplItemMethod { sig, .. }) = item_impl {
						let method_name = sig.ident.clone();

						// generate call method if whitelisted
						if whitelist.contains(&method_name) {
							let call_method_name = format_ident!("method_{}", method_name);
							let args = sig.inputs.iter().collect::<Vec<_>>();
							let inputs = sig.inputs.iter().map(|x| argument_name(&x)).collect::<Vec<_>>();

							// construct call method
							let method = quote! {
								#[pallet::weight(0)]
								pub fn #call_method_name(_origin: OriginFor<T>, #(#args),*) -> DispatchResultWithPostInfo {
									Self::#method_name(#(#inputs),*);
									Ok(().into())
								}
							};

							let call_method: ImplItemMethod = syn::parse(method.into()).unwrap();
							methods.push(ImplItem::from(call_method));
						}
					}
				});
			}
		});

		// Inject methods into #[pallet::call] impl
		let new_content = content
			.into_iter()
			.map(|item| {
				if let Item::Impl(mut item_impl) = item {
					if has_attribute(&item_impl.attrs, "pallet::call") {
						item_impl.items.append(&mut methods);
					}
					return Item::from(item_impl);
				} else {
					item
				}
			})
			.collect::<Vec<Item>>();

		// update content
		item.content = Some((brace, new_content));

		item.into_token_stream().into()
	}
	#[cfg(not(feature = "runtime-benchmarks"))]
	input
}

#[cfg(feature = "runtime-benchmarks")]
fn has_attribute(attrs: &Vec<Attribute>, attr: &str) -> bool {
	if attrs.is_empty() {
		return false;
	}
	let attributes = attrs
		.iter()
		.map(|a| {
			a.path
				.segments
				.iter()
				.map(|p| p.ident.to_string())
				.collect::<Vec<_>>()
				.join("::")
		})
		.collect::<Vec<_>>();
	attributes.contains(&attr.to_string())
}

// Find methods with attribute `#[weight_meter::weight]`
#[cfg(feature = "runtime-benchmarks")]
fn find_methods(content: &Vec<Item>) -> Vec<Ident> {
	let mut methods = vec![];
	content.iter().for_each(|content| {
		if let Item::Impl(item_impl) = content {
			item_impl.items.iter().for_each(|item| {
				if let ImplItem::Method(ImplItemMethod { attrs, sig, .. }) = item {
					if has_attribute(&attrs, "weight_meter::weight") {
						methods.push(sig.ident.clone());
					}
				}
			});
		}
	});
	methods
}

// Extract name from function argument
#[cfg(feature = "runtime-benchmarks")]
fn argument_name(x: &FnArg) -> Box<Pat> {
	match x {
		FnArg::Receiver(_) => panic!("unexpected argument self"),
		FnArg::Typed(ty) => ty.pat.clone(),
	}
}
