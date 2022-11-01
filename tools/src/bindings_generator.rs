use proc_macro2::Span;
use std::{
    fs,
    io::Write,
    iter::FromIterator,
    ops::Deref,
    path::{Path, PathBuf},
};
use syn::{punctuated::Punctuated, Attribute, PathSegment};

pub fn generate_bindings<P: AsRef<Path>>(
    module_file_path: P,
    secure: bool,
) -> Result<(), anyhow::Error> {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    fn generate_bindings_inner<P: AsRef<Path>>(
        module_file_path: P,
        secure: bool,
        generated_items: &mut Vec<(syn::Item, String, u32)>,
    ) -> Result<(), anyhow::Error> {
        println!("cargo:rerun-if-changed={}", module_file_path.as_ref().display());

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
                    generate_bindings_inner(module_entry.path(), secure, generated_items)?;
                }

                if module_entry.path().is_dir() {
                    generate_bindings_inner(
                        module_entry.path().join("mod.rs"),
                        secure,
                        generated_items,
                    )
                    .or_else(|_| {
                        generate_bindings_inner(
                            module_entry
                                .path()
                                .join(module_entry.path().file_name().unwrap())
                                .with_extension("rs"),
                            secure,
                            generated_items,
                        )
                    })?;
                }
            }
        }

        Ok(())
    }

    let mut generated_items = Vec::new();

    generate_bindings_inner(module_file_path, secure, &mut generated_items)?;

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

    let mut output_file = syn::File {
        shebang: None,
        attrs: Vec::new(),
        items: vec![syn::ItemMod {
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
        .into()],
    };

    if secure {
        output_file.items.push(
            syn::parse_str::<syn::ItemFn>(FIND_NS_VECTOR_FUNCTION)
                .unwrap()
                .into(),
        );

        output_file.items.push(
            syn::parse_str::<syn::ItemFn>(FIND_NSC_VECTOR_FUNCTION)
                .unwrap()
                .into(),
        );

        // If we're secure, then we have to create a veneer for the searcher
        output_file.items.push(
            syn::parse_str::<syn::Item>(
                "
                core::arch::global_asm!(
                    \".section .nsc_veneers.searcher, \\\"ax\\\"\",
                    \".global searcher_veneer\",
                    \".thumb_func\",
                    \"searcher_veneer:\",
                        \"SG\",
                        \"B.w find_nsc_veneer\",
                        \".4byte 0\"
                );
            ",
            )
            .unwrap(),
        );

        // If we're secure, then we have to create a function that calls the initializer veneer
        output_file.items.push(syn::parse_str::<syn::Item>(
            "
                #[no_mangle]
                unsafe extern \"C\" fn initialize_ns_data() {
                    extern \"C\" {
                        static _NS_VENEERS: u32;
                    }
                    // Don't forget to set the thumb bit
                    let initializer_veneer_ptr = (&_NS_VENEERS as *const u32 as usize | 1) as *const u32;

                    let initializer_veneer_ptr = core::mem::transmute::<_, extern \"C-cmse-nonsecure-call\" fn()>(initializer_veneer_ptr);
                    initializer_veneer_ptr()
                }
            ").unwrap()
        );
    } else {
        // If we're nonsecure, then we have to create a function that calls the searcher veneer
        output_file.items.push(syn::parse_str::<syn::Item>(
            "
                extern \"C\" fn find_nsc_veneer(hash: u32) -> *const u32 {
                    extern \"C\" {
                        static _NSC_VENEERS: u32;
                    }
                    unsafe {
                        // Don't forget to set the thumb bit
                        let searcher_veneer_ptr = (&_NSC_VENEERS as *const u32 as usize | 1) as *const u32;
                        let searcher_veneer_ptr = core::mem::transmute::<_, extern \"C\" fn(u32) -> *const u32>(searcher_veneer_ptr);
                        searcher_veneer_ptr(hash)
                    }
                }
            ").unwrap()
        );

        // If we're nonsecure, then we have to create a veneer for the initializer
        output_file.items.push(
            syn::parse_str::<syn::Item>(
                "
                core::arch::global_asm!(
                    \".section .ns_veneers.initializer, \\\"ax\\\"\",
                    \".global initializer_veneer\",
                    \".thumb_func\",
                    \"initializer_veneer:\",
                        \"B.w initialize_ns_data\",
                        \".4byte 0\"
                );
            ",
            )
            .unwrap(),
        );
    }

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
                let function_not_found_string = format!("Could not find the veneer of nonsecure '{}'", function_name);

                generated_items.push((
                    syn::ItemFn {
                        attrs: vec![],
                        vis: syn::VisPublic {
                            pub_token: Default::default(),
                        }
                        .into(),
                        sig: signature.clone(),
                        block: Box::new(syn::parse_quote! {
                            {
                                const HASH: u32 = #function_hash;
                                let fn_ptr = unsafe { super::find_ns_veneer(HASH) };

                                if fn_ptr.is_null() {
                                    panic!(#function_not_found_string);
                                }

                                // Don't forget to set the thumb bit
                                let fn_ptr = unsafe {
                                    core::mem::transmute::<_, #function_cast>(((fn_ptr as usize) | 1) as *const u32)
                                };

                                #function_call
                            }
                        }),
                    }
                    .into(),
                    function_name,
                    function_hash,
                ));
            }
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
                let function_not_found_string = format!("Could not find the veneer of secure '{}'", function_name);

                generated_items.push((syn::ItemFn {
                    attrs: vec![],
                    vis: syn::VisPublic {
                        pub_token: Default::default(),
                    }
                    .into(),
                    sig: signature.clone(),
                    block: Box::new(syn::parse_quote! {
                        {
                            const HASH: u32 = #function_hash;
                            let fn_ptr = super::find_nsc_veneer(HASH);

                            if fn_ptr.is_null() {
                                panic!(#function_not_found_string);
                            }

                            // Don't forget to set the thumb bit
                            let fn_ptr = unsafe {
                                core::mem::transmute::<_, #function_cast>(((fn_ptr as usize) | 1) as *const u32)
                            };
                            #function_call
                        }
                    }),
                }
                .into(), function_name, function_hash));
            }
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
    NonSecureCallableFunction { signature: syn::Signature },
}

impl std::fmt::Debug for TrustzoneExportedItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SecureCallableFunction { signature } => f
                .debug_struct("SecureCallableFunction")
                .field("ident", &signature.ident.to_string())
                .finish(),
            Self::NonSecureCallableFunction { signature } => f
                .debug_struct("NonSecureCallableFunction")
                .field("ident", &signature.ident.to_string())
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
unsafe extern \"C\" fn find_ns_veneer(name_hash: u32) -> *const u32 {
    extern \"C\" {
        static _NS_VENEERS: u32;
    }

    let mut ns_veneers_ptr = (&_NS_VENEERS as *const u32 as *const ([u8; 4], u32)).offset(1);

    loop {
        let (_, vector_hash) = *ns_veneers_ptr;

        if vector_hash == 0 {
            // We've reached the end
            return core::ptr::null();
        }

        if vector_hash == name_hash {
            // We've found the vector we've been looking for
            return ns_veneers_ptr as _;
        }

        ns_veneers_ptr = ns_veneers_ptr.offset(1);
    }
}
";

const FIND_NSC_VECTOR_FUNCTION: &'static str = "
#[no_mangle]
#[cmse_nonsecure_entry]
unsafe extern \"C\" fn find_nsc_veneer(name_hash: u32) -> *const u32 {
    extern \"C\" {
        static _NSC_VENEERS: u32;
    }

    let mut nsc_veneers_ptr = (&_NSC_VENEERS as *const u32 as *const ([u8; 8], u32)).offset(1);

    loop {
        let (_, vector_hash) = *nsc_veneers_ptr;

        if vector_hash == 0 {
            // We've reached the end
            return core::ptr::null();
        }

        if vector_hash == name_hash {
            // We've found the vector we've been looking for
            return nsc_veneers_ptr as _;
        }

        nsc_veneers_ptr = nsc_veneers_ptr.offset(1);
    }
}
";
