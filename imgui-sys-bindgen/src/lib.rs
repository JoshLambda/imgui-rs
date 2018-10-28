extern crate bindgen;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use bindgen::{Bindings, EnumVariation, RustTarget};
use failure::Error;
use std::collections::HashMap;
use std::fs::{read_to_string, File};
use std::io::Read;
use std::path::Path;

#[derive(Deserialize)]
struct StructsAndEnums {
    enums: HashMap<String, serde_json::Value>,
    structs: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct DefinitionArg {
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Deserialize)]
struct Definition {
    #[serde(rename = "argsT")]
    args_t: Vec<DefinitionArg>,
}

#[derive(Debug, Clone)]
struct Whitelist {
    enums: Vec<String>,
    structs: Vec<String>,
    definitions: Vec<String>,
}

fn only_key<K, V>((key, _): (K, V)) -> K {
    key
}

fn parse_whitelist<R: Read>(
    structs_and_enums: R,
    definitions: R,
) -> Result<Whitelist, serde_json::Error> {
    let StructsAndEnums { enums, structs } = serde_json::from_reader(structs_and_enums)?;
    let enums = enums.into_iter().map(only_key).collect();
    let structs = structs.into_iter().map(only_key).collect();

    let definitions: HashMap<String, Vec<Definition>> = serde_json::from_reader(definitions)?;
    let definitions = definitions
        .into_iter()
        .filter(|(_, defs)| {
            defs.iter()
                .all(|d| d.args_t.iter().all(|a| a.type_ != "va_list"))
        }).map(only_key)
        .collect();

    Ok(Whitelist {
        enums,
        structs,
        definitions,
    })
}

pub fn generate_bindings<P: AsRef<Path>>(cimgui_path: &P) -> Result<Bindings, Error> {
    let cimgui_output_path = cimgui_path.as_ref().join("generator").join("output");
    let structs_and_enums = File::open(cimgui_output_path.join("structs_and_enums.json"))?;
    let definitions = File::open(cimgui_output_path.join("definitions.json"))?;
    let header = read_to_string(cimgui_output_path.join("cimgui.h"))?;

    let whitelist = parse_whitelist(structs_and_enums, definitions)?;
    let mut builder = bindgen::builder()
        .raw_line("#![allow(non_upper_case_globals)]")
        .raw_line("#![allow(non_camel_case_types)]")
        .raw_line("#![allow(non_snake_case)]")
        .header_contents("cimgui.h", &header)
        .rust_target(RustTarget::Stable_1_26)
        .default_enum_style(EnumVariation::Consts)
        .prepend_enum_name(false)
        .generate_comments(false)
        .layout_tests(true)
        .derive_debug(true)
        .impl_debug(true)
        .rustfmt_bindings(true)
        .clang_arg("-DCIMGUI_DEFINE_ENUMS_AND_STRUCTS=1");
    for e in whitelist.structs {
        builder = builder.whitelist_type(format!("^{}", e));
    }
    for e in whitelist.enums {
        builder = builder.whitelist_type(format!("^{}", e));
    }
    for e in whitelist.definitions {
        builder = builder.whitelist_function(format!("^{}", e));
    }
    let bindings = builder
        .generate()
        .map_err(|_| format_err!("Failed to generate bindings"))?;
    Ok(bindings)
}
