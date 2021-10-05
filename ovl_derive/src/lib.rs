use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse_macro_input;
use syn::Data;
use syn::DeriveInput;
use syn::Ident;

#[proc_macro_derive(FromParsedResource)]
pub fn from_parsed_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let fields_name = match input.data {
        Data::Struct(data_struct) => {
            let mut fields = Vec::new();
            for field in data_struct.fields {
                match field.ident {
                    Some(ident) => fields.push(ident),
                    None => panic!("Field doesn't have ident!"),
                }
            }
            fields
        }
        _ => panic!("FromParsedResource only works for structs"),
    };
    let st_name = input.ident;
    let st_name_string = st_name.to_string();
    let st_name_simple_ident = Ident::new(
        st_name_string.strip_prefix("Ovl").unwrap(),
        Span::call_site(),
    );
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let fields_name_string: Vec<String> = fields_name.iter().map(|f| f.to_string()).collect();

    let token = quote! {
        impl FromParsedResource for #impl_generics #st_name #ty_generics #where_clause {
            fn from_parsed_resource(parsed_resource: &ParsedResource) -> Resource {
                let keys: Vec<String> = parsed_resource.content.iter().map(|k| k.0.clone()).collect();
                let values: Vec<String> = parsed_resource.content.iter().map(|k| k.1.clone()).collect();
                let mut resource = #st_name::default();

                #(
                    if !keys.contains(&#fields_name_string.to_string()) {
                        panic!("Missing field '{}' for struct '{}'", #fields_name_string, #st_name_string);
                    }
                )*

                #(
                    resource.#fields_name = value_from_key(&keys, &values, &#fields_name_string).parse().unwrap();
                )*

                Resource { name: parsed_resource.name.clone(), resource: ResourceType::#st_name_simple_ident(resource) }
            }
        }
    };

    token.into()
}
