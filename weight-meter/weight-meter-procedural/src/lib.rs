extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{ItemFn, FnArg, ImplItem, ImplItemMethod, Item, ItemMod, Attribute};
use quote::{quote, ToTokens};

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
	}).into()
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
	}).into()
}

#[proc_macro_attribute]
pub fn method_benchmarks(_attr: TokenStream, input: TokenStream) -> TokenStream {
    #[cfg(feature = "runtime-benchmarks")] // Inject methods if we're benchmarking
    {
        let mut methods: Vec<ImplItem> = vec![];

        let mut item: ItemMod = syn::parse(input).unwrap();
        let (brace, content) = item.content.clone().unwrap();

        let method_names = find_methods(&content);

        // Generate methods dynamically
        content.iter().for_each(|i| {
            if let Item::Impl(x) = i { // implementation
                x.items.iter().for_each(|x| {
                    if let ImplItem::Method(i) = x { // method
                        let method_name = i.sig.ident.to_string();
                        if method_names.contains(&method_name) {
                            let call_method_name: proc_macro2::TokenStream = i.sig.ident.to_string().parse().unwrap();
                            let inject_method: proc_macro2::TokenStream = format!("method_{}", method_name).parse().unwrap();
                            let args = i.sig.inputs.clone().into_iter().collect::<Vec<_>>();
                            let inputs: Vec<proc_macro2::TokenStream> = i.sig.inputs
                                .iter()
                                .map(|x| argument_name(&x))
                                .filter(|x| x != &"")
                                .map(|x| x.parse().unwrap())
                                .collect();

                            let method = quote! {
                                #[pallet::weight(0)]
                                pub fn #inject_method(_origin: OriginFor<T>, #(#args),*) -> DispatchResultWithPostInfo {
                                    Self::#call_method_name(#(#inputs),*);
                                    Ok(().into())
                                }
                            };

                            let generated_method: ImplItemMethod = syn::parse(method.into()).unwrap();
                            methods.push(ImplItem::from(generated_method));
                        }
                    }
                });
            }
        });

        // Inject methods into pallet::call impl
        let new_content = content.into_iter().map(|item| {
            if let Item::Impl(mut item_impl) = item {
                if has_attribute(&item_impl.attrs, "pallet::call") {
                    println!("injected callable methods for inner methods {:?}", method_names);
                    item_impl.items.append(&mut methods);
                }
                return Item::from(item_impl);
            } else {
                item
            }
        }).collect::<Vec<Item>>();

        // update content
        item.content = Some((brace, new_content));

        item.into_token_stream().into()
    }
    #[cfg(not(feature = "runtime-benchmarks"))]
    input
}

#[cfg(feature = "runtime-benchmarks")]
fn has_attribute(attrs: &Vec<Attribute>, attr: &str) -> bool {
    if attrs.is_empty() { return false };
    let attributes = attrs.iter().map(|a| {
        a.path.segments.iter().map(|p| p.ident.to_string()).collect::<Vec<_>>().join("::")
    }).collect::<Vec<_>>();
    attributes.contains(&attr.to_string())
}

#[cfg(feature = "runtime-benchmarks")]
fn find_methods(content: &Vec<Item>) -> Vec<String> {
    let mut method_names = vec![];
    content.iter().for_each(|i| {
        if let Item::Impl(x) = i {
            x.items.iter().for_each(|x| {
                if let ImplItem::Method(i) = x {
                    if has_attribute(&i.attrs, "weight_meter::weight") {
                        method_names.push(i.sig.ident.to_string());
                    }
                }
            })
        }
    });
    method_names
}

#[cfg(feature = "runtime-benchmarks")]
fn argument_name(x: &FnArg) -> String {
    match x {
        FnArg::Receiver(_) => "".into(),
        FnArg::Typed(a) => a.pat.to_token_stream().to_string(),
    }
}