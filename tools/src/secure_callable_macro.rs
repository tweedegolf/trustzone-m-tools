use proc_macro2::{Span, TokenStream};
use syn::parse2;

pub fn secure_callable(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse2::<syn::ItemFn>(item);

    let function = match function {
        Ok(f) => f,
        Err(e) => {
            return e.into_compile_error();
        }
    };
    let function = &function;

    if !function.sig.abi.as_ref().map_or(false, |abi| {
        abi.name
            .as_ref()
            .map_or(false, |abi_name| abi_name.value().to_uppercase() == "C")
    }) {
        return quote::quote! {
            compile_error!("Function must be 'extern \"C\"'");
        };
    }

    let function_name = function.sig.ident.to_string();
    let function_name_ident = syn::Ident::new(&function_name, Span::call_site());
    let function_vector_name = syn::Ident::new(
        &format!("{}_VECTOR", function_name.to_uppercase()),
        Span::call_site(),
    );
    let vector_name_hash = crate::hash_vector_name(&function_name);
    let function_ptr_type = syn::TypeBareFn {
        lifetimes: None,
        unsafety: function.sig.unsafety,
        abi: function.sig.abi.clone(),
        fn_token: function.sig.fn_token,
        paren_token: function.sig.paren_token,
        inputs: function
            .sig
            .inputs
            .iter()
            .filter_map(|arg| {
                if let syn::FnArg::Typed(t) = arg {
                    Some(t)
                } else {
                    None
                }
            })
            .map(|pat_type| syn::BareFnArg {
                attrs: pat_type.attrs.clone(),
                name: None,
                ty: *pat_type.ty.clone(),
            })
            .collect(),
        variadic: function.sig.variadic.clone(),
        output: function.sig.output.clone(),
    };

    quote::quote! {
        #[link_section = ".vectors"]
        #[no_mangle]
        #[used]
        static #function_vector_name: (#function_ptr_type, u32) = (#function_name_ident, #vector_name_hash);

        #[link_section = ".text.exported"]
        #[no_mangle]
        #function
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_name() {
        let input_text = include_str!("../test-sources/secure_callable_simple_test.txt");
        let output_text = include_str!("../test-sources/secure_callable_simple_result.txt");

        let attr: String = input_text.lines().take(1).collect();
        let item: String = input_text.lines().skip(1).collect();

        let attr_stream = TokenStream::from_str(&attr).unwrap();
        let item_stream = TokenStream::from_str(&item).unwrap();

        let output = secure_callable(attr_stream, item_stream);

        let pretty_output = prettyplease::unparse(&parse2(output).unwrap());
        pretty_assertions::assert_eq!(
            pretty_output.replace("\r\n", "\n"),
            output_text.replace("\r\n", "\n")
        );
    }
}
