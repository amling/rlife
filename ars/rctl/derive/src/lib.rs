extern crate proc_macro;
#[macro_use]
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::FnArg;
use syn::Ident;
use syn::ImplItem;
use syn::ItemImpl;
use syn::LitStr;
use syn::Pat;
use syn::ReturnType;
use syn::export::Span;

#[proc_macro_attribute]
pub fn rctl_ep(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemImpl);

    let mut metadatas = Vec::new();
    let mut matches = Vec::new();
    for item in item.items.iter() {
        if let ImplItem::Method(method) = item {
            let sig = &method.sig;
            let ident = &sig.ident;
            let ident_str = LitStr::new(&format!("{}", ident), Span::call_site());
            let mut arg_metadatas = Vec::new();
            let mut arg_lets = Vec::new();
            let mut arg_exprs = Vec::new();
            for (n, input) in sig.decl.inputs.iter().enumerate() {
                match input {
                    FnArg::SelfRef(_) => {
                    },
                    FnArg::Captured(input) => {
                        let arg_var = Ident::new(&format!("arg{}", n), Span::call_site());
                        let arg_name = match &input.pat {
                            Pat::Ident(input) => format!("{}", input.ident),
                            _ => panic!(),
                        };
                        let arg_str = LitStr::new(&arg_name, Span::call_site());
                        let arg_ty = &input.ty;
                        arg_metadatas.push(quote! {
                            <#arg_ty as ars_rctl_core::RctlArgTrait>::add_metadata(&mut args, #arg_str);
                        });
                        arg_lets.push(quote! {
                            let #arg_var = ars_rctl_core::RctlArgTrait::take_arg(&mut args).map_err(|e| ars_ds::err::StringError::from(e).label(format!("While parsing {}", #arg_str)))?;
                        });
                        arg_exprs.push(quote! {
                            #arg_var,
                        });
                    },
                    _ => panic!(),
                }
            }
            let ret_type = match &sig.decl.output {
                ReturnType::Default => quote! { ars_rctl_core::RctlType::of::<()>() },
                ReturnType::Type(_, ty) => quote! { ars_rctl_core::RctlType::of::<#ty>() },
            };
            metadatas.push(quote! {
                (
                    #ident_str.to_string(),
                    ars_rctl_core::RctlMethodMetadata {
                        args: {
                            let mut args = Vec::new();
                            #( #arg_metadatas )*
                            args
                        },
                        ret: #ret_type,
                    },
                ),
            });
            matches.push(quote! {
                #ident_str => {
                    let mut args = ars_rctl_core::RctlArgsBag::new(args, log);
                    #( #arg_lets )*
                    if !args.is_done() {
                        return Err(ars_ds::err::StringError::new("Leftover arguments"));
                    }
                    let ret = self.#ident(#( #arg_exprs )*);
                    let ret = serde_json::to_value(ret).map_err(|e| ars_ds::err::StringError::from(e).label("While deparsing return value"))?;
                    return Ok(ret);
                }
            });
        }
    }

    let (impl_generics, _, where_clause) = item.generics.split_for_impl();
    let self_ty = &item.self_ty;
    let imp = quote! {
        impl #impl_generics ars_rctl_core::RctlEp for #self_ty #where_clause {
            fn metadata() -> Vec<(String, ars_rctl_core::RctlMethodMetadata)> {
                vec![
                    #( #metadatas )*
                ]
            }

            fn invoke(&self, log: ars_rctl_core::RctlLog, method: impl AsRef<str>, args: &[serde_json::value::Value]) -> Result<serde_json::value::Value, ars_ds::err::StringError> {
                let method = method.as_ref();
                match method {
                    #( #matches )*
                    _ => Err(ars_ds::err::StringError::new(format!("No such method {}", method))),
                }
            }
        }
    };

    let ret = quote! {
        #item

        #imp
    };

    TokenStream::from(ret)
}
