use proc_macro2::TokenStream;

pub fn secure_callable(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr: \"{}\"", attr.to_string());
    println!("item: \"{}\"", item.to_string());
    item
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_name() {
        let attr = r#"
#[secure_callable]
        "#;
        let item = r#"
extern "C" fn test() {}
        "#;


        let attr_stream = TokenStream::from_str(attr).unwrap();
        let item_stream = TokenStream::from_str(item).unwrap();

        secure_callable(attr_stream, item_stream);
    }
}
