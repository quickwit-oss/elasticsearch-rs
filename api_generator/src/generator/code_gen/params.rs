/*
 * Licensed to Elasticsearch B.V. under one or more contributor
 * license agreements. See the NOTICE file distributed with
 * this work for additional information regarding copyright
 * ownership. Elasticsearch B.V. licenses this file to you under
 * the Apache License, Version 2.0 (the "License"); you may
 * not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *	http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */
use crate::generator::code_gen::stability_doc;
use crate::generator::*;
use inflector::Inflector;
use quote::Tokens;
use regex::Regex;

pub fn generate(api: &Api) -> anyhow::Result<String> {
    let mut tokens = Tokens::new();

    for e in &api.enums {
        generate_param(&mut tokens, &e);
    }

    let generated = tokens.to_string();
    Ok(generated)
}

fn generate_param(tokens: &mut Tokens, e: &ApiEnum) {
    let name = syn::Ident::from(e.name.to_pascal_case());
    let (renames, variants): (Vec<String>, Vec<syn::Ident>) = e
        .values
        .iter()
        .map(|v| {
            if v.is_empty() {
                (v.to_owned(), syn::Ident::from("Unspecified"))
            } else if !v.contains('(') {
                (v.to_owned(), syn::Ident::from(v.to_pascal_case()))
            } else {
                lazy_static! {
                    static ref PARENS_REGEX: Regex = Regex::new(r"^(.*?)\s*\(.*?\)\s*$").unwrap();
                }
                if let Some(c) = PARENS_REGEX.captures(v) {
                    (
                        c.get(1).unwrap().as_str().to_owned(),
                        syn::Ident::from(c.get(1).unwrap().as_str().to_pascal_case()),
                    )
                } else {
                    (v.to_owned(), syn::Ident::from(v.to_pascal_case()))
                }
            }
        })
        .unzip();

    let doc = match &e.description {
        Some(description) => Some(code_gen::doc(description)),
        None => None,
    };

    let cfg_attr = e.stability.outer_cfg_attr();
    let cfg_doc = stability_doc(e.stability);

    let mut from_str_branches = quote!();
    for (rename, variant) in renames.iter().zip(variants.iter()) {
        from_str_branches.append(quote!( #rename => Ok(Self::#variant), ))
    }

    let mut to_str_branches = quote!();
    for (rename, variant) in renames.iter().zip(variants.iter()) {
        to_str_branches.append(quote!( Self::#variant => #rename.to_string(), ))
    }

    let generated_enum_tokens = quote!(
        #doc
        #cfg_doc
        #cfg_attr
        #[derive(Debug, PartialEq, Deserialize, Serialize, Clone, Copy)]
        pub enum #name {
            #(#[serde(rename = #renames)] #variants),*
        }

        impl FromStr for #name {
            type Err = &'static str;
            fn from_str(value_str: &str) -> Result<Self, Self::Err> {
                match value_str {
                    #from_str_branches
                    _ => Err("unknown enum variant")
                }
            }
        }

        impl ToString for #name {
            fn to_string(&self) -> String {
                match &self {
                    #to_str_branches
                }
            }
        }
    );

    tokens.append(generated_enum_tokens);
}
