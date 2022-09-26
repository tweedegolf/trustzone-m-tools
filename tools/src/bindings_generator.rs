use proc_macro2::Span;
use std::{
    fs,
    io::Write,
    iter::FromIterator,
    ops::Deref,
    path::{Path, PathBuf},
};
use syn::{punctuated::Punctuated, Attribute, PathSegment};

pub fn generate_bindings<P: AsRef<Path>>(module_file_path: P) -> Result<(), anyhow::Error> {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    fn generate_bindings_inner<P: AsRef<Path>>(
        module_file_path: P,
        generated_items: &mut Vec<(syn::Item, String, u32)>,
    ) -> Result<(), anyhow::Error> {
        // Read the source code file
        let file_text = std::fs::read_to_string(module_file_path.as_ref())?;

        // Parse the file
        let file = syn::parse_file(&file_text)?;

        // Generate the bindings to the file
        let child_modules = generate_file_bindings(file, generated_items)?;

        // Continue reading other modules
        for module in child_modules {
            // Is this a module that refers to another file?
            if module.content.is_some() {
                // No, so let's continue
                continue;
            }

            let module_name = module.ident.to_string();

            let module_entry = module_file_path
                .as_ref()
                .parent()
                .unwrap()
                .read_dir()?
                .filter_map(Result::ok)
                .find(|entry| {
                    entry
                        .path()
                        .with_extension("")
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        == module_name
                });

            if let Some(module_entry) = module_entry {
                if module_entry.path().is_file() {
                    generate_bindings_inner(module_entry.path(), generated_items)?;
                }

                if module_entry.path().is_dir() {
                    generate_bindings_inner(module_entry.path().join("mod.rs"), generated_items)
                        .or_else(|_| {
                            generate_bindings_inner(
                                module_entry
                                    .path()
                                    .join(module_entry.path().file_name().unwrap())
                                    .with_extension("rs"),
                                generated_items,
                            )
                        })?;
                }
            }
        }

        Ok(())
    }

    let mut generated_items = Vec::new();

    generate_bindings_inner(module_file_path, &mut generated_items)?;

    // Check if there aren't any name and hash collisions
    for (_, name, hash) in generated_items.iter() {
        assert_eq!(
            generated_items
                .iter()
                .filter(|(_, other_name, _)| name == other_name)
                .count(),
            1,
            "Duplicate name found: {name}"
        );
        for (_, other_name, other_hash) in generated_items.iter() {
            if name != other_name {
                assert_ne!(hash, other_hash, "Hash collision found for `{name}` and `{other_name}`. To fix this, change one of the names. This is a limitation of how the trustzone-m-tools work.");
            }
        }
    }

    let output_file = syn::File {
        shebang: None,
        attrs: Vec::new(),
        items: vec![
            syn::ItemMod {
                attrs: Vec::new(),
                vis: syn::VisPublic {
                    pub_token: Default::default(),
                }
                .into(),
                mod_token: Default::default(),
                ident: syn::Ident::new("trustzone_bindings", Span::call_site()),
                content: Some((
                    syn::token::Brace::default(),
                    generated_items
                        .into_iter()
                        .map(|(item, _, _)| item)
                        .collect(),
                )),
                semi: None,
            }
            .into(),
            syn::parse_str::<syn::ItemFn>(FIND_NS_VECTOR_FUNCTION)
                .unwrap()
                .into(),
            syn::parse_str::<syn::ItemFn>(FIND_NSC_VECTOR_FUNCTION)
                .unwrap()
                .into(),
        ],
    };

    let mut output_bindings_file =
        fs::File::create(PathBuf::from(out_dir).join("trustzone_bindings.rs"))?;

    output_bindings_file.write_all(prettyplease::unparse(&output_file).as_bytes())?;

    Ok(())
}

fn generate_file_bindings(
    file: syn::File,
    generated_items: &mut Vec<(syn::Item, String, u32)>,
) -> Result<Vec<syn::ItemMod>, anyhow::Error> {
    let found_exported_items = TrustzoneExportedItem::find(file.items.iter());

    for exported_item in found_exported_items {
        match exported_item {
            TrustzoneExportedItem::SecureCallableFunction { signature } => {
                let function_call = syn::ExprCall {
                    attrs: Vec::new(),
                    func: Box::new(
                        syn::ExprPath {
                            attrs: Vec::new(),
                            qself: None,
                            path: syn::Path {
                                leading_colon: None,
                                segments: Punctuated::from_iter([PathSegment {
                                    ident: syn::Ident::new("fn_ptr", Span::call_site()),
                                    arguments: syn::PathArguments::None,
                                }]),
                            },
                        }
                        .into(),
                    ),
                    paren_token: Default::default(),
                    args: Punctuated::from_iter(
                        signature
                            .inputs
                            .iter()
                            .filter_map(|input| match input {
                                syn::FnArg::Typed(t) => Some(t),
                                _ => None,
                            })
                            .map(|input| match input.pat.deref() {
                                syn::Pat::Ident(i) => syn::Expr::Path(syn::ExprPath {
                                    attrs: Vec::new(),
                                    qself: None,
                                    path: syn::Path {
                                        leading_colon: None,
                                        segments: Punctuated::from_iter([PathSegment {
                                            ident: i.ident.clone(),
                                            arguments: syn::PathArguments::None,
                                        }]),
                                    },
                                }),
                                _ => unreachable!(),
                            }),
                    ),
                };

                let function_cast = syn::TypeBareFn {
                    lifetimes: None,
                    unsafety: signature.unsafety.clone(),
                    abi: Some(syn::Abi {
                        extern_token: Default::default(),
                        name: Some(syn::LitStr::new("C-cmse-nonsecure-call", Span::call_site())),
                    }),
                    fn_token: Default::default(),
                    paren_token: Default::default(),
                    inputs: signature
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
                    variadic: signature.variadic.clone(),
                    output: signature.output.clone(),
                };

                let function_name = signature.ident.to_string();
                let function_hash = crate::hash_vector_name(&function_name);

                generated_items.push((syn::ItemFn {
                    attrs: vec![
                        syn::parse_quote! { #[inline(never)] }
                    ],
                    vis: syn::VisPublic {
                        pub_token: Default::default(),
                    }
                    .into(),
                    sig: signature.clone(),
                    block: Box::new(syn::parse_quote! {
                        {
                            const HASH: u32 = #function_hash;
                            let fn_ptr = unsafe { super::find_ns_vector::<#function_cast>(HASH).unwrap() };
                            #function_call
                        }
                    }),
                }
                .into(), function_name, function_hash));
            }
            TrustzoneExportedItem::SecureCallableStatic {
                name: _,
                item_type: _,
            } => todo!(),
            TrustzoneExportedItem::NonSecureCallableFunction { signature } => {
                let function_call = syn::ExprCall {
                    attrs: Vec::new(),
                    func: Box::new(
                        syn::ExprPath {
                            attrs: Vec::new(),
                            qself: None,
                            path: syn::Path {
                                leading_colon: None,
                                segments: Punctuated::from_iter([PathSegment {
                                    ident: syn::Ident::new("fn_ptr", Span::call_site()),
                                    arguments: syn::PathArguments::None,
                                }]),
                            },
                        }
                        .into(),
                    ),
                    paren_token: Default::default(),
                    args: Punctuated::from_iter(
                        signature
                            .inputs
                            .iter()
                            .filter_map(|input| match input {
                                syn::FnArg::Typed(t) => Some(t),
                                _ => None,
                            })
                            .map(|input| match input.pat.deref() {
                                syn::Pat::Ident(i) => syn::Expr::Path(syn::ExprPath {
                                    attrs: Vec::new(),
                                    qself: None,
                                    path: syn::Path {
                                        leading_colon: None,
                                        segments: Punctuated::from_iter([PathSegment {
                                            ident: i.ident.clone(),
                                            arguments: syn::PathArguments::None,
                                        }]),
                                    },
                                }),
                                _ => unreachable!(),
                            }),
                    ),
                };

                let function_cast = syn::TypeBareFn {
                    lifetimes: None,
                    unsafety: signature.unsafety.clone(),
                    abi: signature.abi.clone(),
                    fn_token: Default::default(),
                    paren_token: Default::default(),
                    inputs: signature
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
                    variadic: signature.variadic.clone(),
                    output: signature.output.clone(),
                };

                let function_name = signature.ident.to_string();
                let function_hash = crate::hash_vector_name(&function_name);

                generated_items.push((syn::ItemFn {
                    attrs: vec![
                        syn::parse_quote! { #[inline(never)] }
                    ],
                    vis: syn::VisPublic {
                        pub_token: Default::default(),
                    }
                    .into(),
                    sig: signature.clone(),
                    block: Box::new(syn::parse_quote! {
                        {
                            const HASH: u32 = #function_hash;
                            let fn_ptr = unsafe { super::find_nsc_vector::<#function_cast>(HASH).unwrap() };
                            #function_call
                        }
                    }),
                }
                .into(), function_name, function_hash));
            },
            TrustzoneExportedItem::NonSecureCallableStatic {
                name: _,
                item_type: _,
            } => todo!(),
        }
    }

    let found_modules = file
        .items
        .iter()
        // Get all modules
        .filter_map(|item| match item {
            syn::Item::Mod(module) => Some(module),
            _ => None,
        })
        // Keep only the modules that refer to other files
        .filter(|module| module.content.is_none())
        .cloned()
        .collect();

    Ok(found_modules)
}

#[allow(dead_code)]
enum TrustzoneExportedItem {
    SecureCallableFunction { signature: syn::Signature },
    SecureCallableStatic { name: String, item_type: syn::Type },
    NonSecureCallableFunction { signature: syn::Signature },
    NonSecureCallableStatic { name: String, item_type: syn::Type },
}

impl std::fmt::Debug for TrustzoneExportedItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SecureCallableFunction { signature } => f
                .debug_struct("SecureCallableFunction")
                .field("ident", &signature.ident.to_string())
                .finish(),
            Self::SecureCallableStatic { name, item_type: _ } => f
                .debug_struct("SecureCallableStatic")
                .field("ident", name)
                .finish(),
            Self::NonSecureCallableFunction { signature } => f
                .debug_struct("NonSecureCallableFunction")
                .field("ident", &signature.ident.to_string())
                .finish(),
            Self::NonSecureCallableStatic { name, item_type: _ } => f
                .debug_struct("NonSecureCallableStatic")
                .field("ident", name)
                .finish(),
        }
    }
}

impl TrustzoneExportedItem {
    fn find<'i, I: IntoIterator<Item = &'i syn::Item>>(items: I) -> Vec<TrustzoneExportedItem> {
        let mut exported_items = Vec::new();

        fn find_inner<'i>(
            items: &mut dyn Iterator<Item = &'i syn::Item>,
            exported_items: &mut Vec<TrustzoneExportedItem>,
        ) {
            for item in items {
                match item {
                    syn::Item::Fn(function) => {
                        if contains_secure_callable_attr(&function.attrs) {
                            exported_items.push(TrustzoneExportedItem::SecureCallableFunction {
                                signature: function.sig.clone(),
                            });
                        }
                        if contains_nonsecure_callable_attr(&function.attrs) {
                            exported_items.push(TrustzoneExportedItem::NonSecureCallableFunction {
                                signature: function.sig.clone(),
                            });
                        }

                        find_inner(
                            &mut function.block.stmts.iter().filter_map(|stmt| match stmt {
                                syn::Stmt::Item(item) => Some(item),
                                _ => None,
                            }),
                            exported_items,
                        );
                    }
                    syn::Item::Impl(implementation) => {
                        for impl_item in implementation.items.iter() {
                            match impl_item {
                                syn::ImplItem::Method(method) => {
                                    if contains_secure_callable_attr(&method.attrs) {
                                        exported_items.push(
                                            TrustzoneExportedItem::SecureCallableFunction {
                                                signature: method.sig.clone(),
                                            },
                                        );
                                    }
                                    if contains_nonsecure_callable_attr(&method.attrs) {
                                        exported_items.push(
                                            TrustzoneExportedItem::NonSecureCallableFunction {
                                                signature: method.sig.clone(),
                                            },
                                        );
                                    }

                                    find_inner(
                                        &mut method.block.stmts.iter().filter_map(
                                            |stmt| match stmt {
                                                syn::Stmt::Item(item) => Some(item),
                                                _ => None,
                                            },
                                        ),
                                        exported_items,
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    syn::Item::Static(_) => {
                        // TODO
                    }
                    _ => {}
                }
            }
        }

        find_inner(&mut items.into_iter(), &mut exported_items);

        exported_items
    }
}

fn contains_secure_callable_attr(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .any(|attr| attr.path.segments.last().unwrap().ident.to_string() == "secure_callable")
}

fn contains_nonsecure_callable_attr(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .any(|attr| attr.path.segments.last().unwrap().ident.to_string() == "nonsecure_callable")
}

const FIND_NS_VECTOR_FUNCTION: &'static str = "
#[allow(dead_code)]
#[inline(never)]
unsafe fn find_ns_vector<F>(name_hash: u32) -> Option<F> {
    extern \"C\" {
        static _NS_VECTORS: u32;
    }

    let mut ns_vectors_ptr = &_NS_VECTORS as *const u32 as *const (u32, u32);

    loop {
        let (vector, vector_hash) = *ns_vectors_ptr;

        if vector == 0 && vector_hash == 0 {
            // We've reached the end
            return None;
        }

        if vector_hash == name_hash {
            // We've found the vector we've been looking for
            return Some(core::mem::transmute_copy(&vector));
        }

        ns_vectors_ptr = ns_vectors_ptr.offset(1);
    }
}
";

const FIND_NSC_VECTOR_FUNCTION: &'static str = "
#[allow(dead_code)]
#[inline(never)]
unsafe fn find_nsc_vector<F>(name_hash: u32) -> Option<F> {
    extern \"C\" {
        static _NSC_VECTORS: u32;
    }

    let mut nsc_vectors_ptr = &_NSC_VECTORS as *const u32 as *const (u32, u32);

    loop {
        let (vector, vector_hash) = *nsc_vectors_ptr;

        if vector == 0 && vector_hash == 0 {
            // We've reached the end
            return None;
        }

        if vector_hash == name_hash {
            // We've found the vector we've been looking for
            return Some(core::mem::transmute_copy(&vector));
        }

        nsc_vectors_ptr = nsc_vectors_ptr.offset(1);
    }
}
";
