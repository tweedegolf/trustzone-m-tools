use proc_macro2::TokenStream;
use syn::parse2;

pub fn nonsecure_callable(_attr: TokenStream, item: TokenStream) -> TokenStream {
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
    let function_veneer_name = format!("{}_veneer", function_name.to_uppercase());
    let function_name_hash = crate::hash_vector_name(&function_name);

    let global = format!(".global {function_veneer_name}");
    let label = format!("{function_veneer_name}:");
    let branch = format!("B.w {function_name}");
    let hash = format!(".4byte {function_name_hash}");

    quote::quote! {
        core::arch::global_asm!(
            ".section .nsc_veneers, \"ax\"",
            #global,
            ".thumb_func",
            #label,
                "SG",
                #branch,
                #hash,
        );

        #[cmse_nonsecure_entry]
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
        let input_text = include_str!("../test-sources/nonsecure_callable_simple_test.txt");
        let output_text = include_str!("../test-sources/nonsecure_callable_simple_result.txt");

        let attr: String = input_text.lines().take(1).collect();
        let item: String = input_text.lines().skip(1).collect();

        let attr_stream = TokenStream::from_str(&attr).unwrap();
        let item_stream = TokenStream::from_str(&item).unwrap();

        let output = nonsecure_callable(attr_stream, item_stream);

        let pretty_output = prettyplease::unparse(&parse2(output).unwrap());
        pretty_assertions::assert_eq!(
            pretty_output.replace("\r\n", "\n"),
            output_text.replace("\r\n", "\n")
        );
    }
}
