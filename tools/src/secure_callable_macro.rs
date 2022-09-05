use proc_macro2::TokenStream;
use syn::parse2;

pub fn secure_callable(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse2::<syn::ItemFn>(dbg!(item));

    let function = match function {
        Ok(f) => f,
        Err(e) => {
            return e.into_compile_error();
        }
    };

    quote::quote! {
        #[link_section = ".vectors"]
        #[used]
        static READ_THING_VECTOR: (extern "C" fn() -> u32, u32) = (read_thing, hash("read_thing"));

        #[link_section = ".text.exported"]
        #function
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_name() {
        let text = include_str!("../test-sources/test_secure_callable.txt");

        let attr: String = text.lines().take(1).collect();
        let item: String = text.lines().skip(1).collect();

        let attr_stream = TokenStream::from_str(&attr).unwrap();
        let item_stream = TokenStream::from_str(&item).unwrap();

        let output = secure_callable(attr_stream, item_stream);

        println!("{}", output.to_string());
    }
}
