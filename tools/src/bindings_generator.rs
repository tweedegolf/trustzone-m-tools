use std::{error::Error, path::Path};

pub fn generate_bindings<P: AsRef<Path>>(module_file_path: P) -> Result<(), Box<dyn Error>> {
    let module_text = std::fs::read_to_string(module_file_path.as_ref())?;

    let file = syn::parse_file(&module_text)?;

    for module in generate_file_bindings(file)? {
        if module.content.is_some() {
            continue;
        }

        let module_name = module.ident.to_string();

        // TODO: Add module folders
        let module_entry = module_file_path.as_ref().read_dir()?.filter_map(Result::ok).find(|entry| entry.file_name().to_string_lossy() == format!("{}.rs", module_name));

        if let Some(module_entry) = module_entry {
            if module_entry.path().is_file() {
                generate_bindings(module_entry.path())?;
            }

            if module_entry.path().is_dir() {
                // TODO
            }
        }
    }

    Ok(())
}

fn generate_file_bindings(file: syn::File) -> Result<Vec<syn::ItemMod>, Box<dyn Error>> {
    todo!()
}
