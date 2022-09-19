use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn secure_callable(attr: TokenStream, item: TokenStream) -> TokenStream {
    trustzone_m_tools::secure_callable_macro::secure_callable(attr.into(), item.into()).into()
}

#[proc_macro_attribute]
pub fn nonsecure_callable(attr: TokenStream, item: TokenStream) -> TokenStream {
    trustzone_m_tools::nonsecure_callable_macro::nonsecure_callable(attr.into(), item.into()).into()
}

