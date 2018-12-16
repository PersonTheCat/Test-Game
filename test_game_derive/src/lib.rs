#![recursion_limit="128"]

#[macro_use]
extern crate quote;
extern crate proc_macro;

use proc_macro::TokenStream;
use syn::DeriveInput;
use syn::Data::*;
use syn::Field;
use syn::Type::*;

#[proc_macro_derive(AtomicClone)]
pub fn atomic_clone(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_atomic_clone(&ast)
}

fn impl_atomic_clone(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let mut clones = Vec::new();
    if let Struct(ref data) = ast.data {
        data.fields.iter()
            .for_each(|f|
                clones.push(clone_field(f))
            )
    }

    let expanded = quote! {
        use crate::traits::AtomicClone;

        impl AtomicClone for #name {}

        impl Clone for #name {
            fn clone(&self) -> Self {
                #name {
                    #(#clones),*
                }
            }
        }
    };
    expanded.into()
}

fn clone_field(field: &Field) -> proc_macro2::TokenStream {
    let ident = &field.ident;
    let ty = &field.ty;
    match ty {
        Path(path) => {
            let ty_str = path.path.segments
                .first() // First type parameter
                .unwrap() // Must exist
                .value() // Value, ignoring separator
                .ident // Identifier
                .to_string();

            match ty_str.as_str() {
                "Atomic" => {
                    quote! {
                        #ident: Atomic::new(self.#ident.load(SeqCst).clone())
                    }
                },
                "Mutex" => {
                    quote! {
                        #ident: Mutex::new(self.#ident.lock().clone())
                    }
                },
                "RwLock" => {
                    quote! {
                        #ident: RwLock::new(self.#ident.read().clone())
                    }
                }
                _ => quote! { #ident: self.#ident.clone() }
            }
        },
        _ => quote! { #ident: self.#ident.clone() }
    }
}

#[proc_macro_derive(ItemTools)]
pub fn item_tools(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_item_tools(&ast)
}

fn impl_item_tools(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let expanded = quote! {
        use crate::traits::ItemTools;
        use std::any::Any;

        impl ItemTools for #name {
            fn clone_box(&self) -> Box<Item> { Box::new(self.clone()) }

            fn as_any(&self) -> &Any { self }
        }
    };
    expanded.into()
}

#[proc_macro_derive(AreaTools)]
pub fn area_tools(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_area_tools(&ast)
}

fn impl_area_tools(ast: &DeriveInput) -> TokenStream {
    if !has_field(ast, "coordinates") {
        panic!("Error: You must provide a field for coordinates when using #[derive(AreaTools)].");
    }
    if !has_field(ast, "area_num") {
        panic!("Error: You must provide a field for area_num when using #[derive(AreaTools)].");
    }
    if !has_field(ast, "connections") {
        panic!("Error: You must provide a field for connections when using #[derive(AreaTools)].");
    }

    let name = &ast.ident;

    let expanded = quote! {
        use crate::traits::AreaTools;
        use std::any::Any;

        impl AreaTools for #name {
            fn get_area_num(&self) -> usize { self.area_num }

            fn get_coordinates(&self) -> (usize, usize, usize) { self.coordinates }

            fn get_town_num(&self) -> usize { self.coordinates.0 }

            fn add_connection(&self, connection: (usize, usize, usize)) {
                self.connections.lock().push(connection);
            }

            fn get_connections(&self) -> Vec<(usize, usize, usize)> {
                self.connections.lock().to_vec()
            }

            fn as_entity_holder(&self) -> &EntityHolder { self }

            fn as_any(&self) -> &Any { self }
        }
    };
    expanded.into()
}

#[proc_macro_derive(EntityHolder)]
pub fn entity_holder(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_entity_holder(&ast)
}

fn impl_entity_holder(ast: &DeriveInput) -> TokenStream {
    if !has_field(ast, "entities") {
        panic!("Error: You must provide a field for entities when using #[derive(EntityHolder)].");
    }
    if !has_field(ast, "coordinates") {
        panic!("Error: You must provide a field for coordinates when using #[derive(EntityHolder)].");
    }

    let name = &ast.ident;

    let expanded = quote! {
        use crate::traits::EntityHolder;
        use parking_lot::RwLockReadGuard;

        impl EntityHolder for #name {
            fn contains_type(&self, typ: &str) -> bool {
                let entities = self.entities.read();
                for entity in entities.iter() {
                    if entity.get_type() == typ {
                        return true;
                    }
                }
                false
            }

            fn add_entity(&self, entity: Box<Entity>) {
                entity.on_enter_area(self.coordinates);
                self.entities.write().push(entity);
            }

            fn remove_entity(&self, id: usize) -> Option<Box<Entity>> {
                if let Some(num) = self.get_entity_index(id) {
                    return Some(self.take_entity_by_index(num));
                }
                None
            }

            fn transfer_entity(&self, id: usize, to: &EntityHolder) {
                let entity = self.remove_entity(id)
                    .expect("Error: Attempted to remove entity who no longer existed in area.");

                to.add_entity(entity);
            }

            fn contains_entity(&self, id: usize) -> bool {
                self.get_entity_index(id).is_some()
            }

            fn get_entity_index(&self, id: usize) -> Option<usize> {
                self.entities.read()
                    .iter()
                    .position(|e| { e.get_id() == id })
            }

            fn take_entity_by_index(&self, index: usize) -> Box<Entity> {
                self.entities.write().remove(index)
            }

            fn borrow_entity_lock(&self) -> RwLockReadGuard<Vec<Box<Entity>>> {
                self.entities.read()
            }
        }
    };
    expanded.into()
}

fn has_field(ast: &DeriveInput, name: &str) -> bool {
    access_field(ast, name, |_|{}).is_some()
}

fn access_field<T, F>(ast: &DeriveInput, name: &str, callback: F) -> Option<T> where F: FnOnce(&Field) -> T {
    if let Struct(ref data) = ast.data {
        for field in data.fields.iter() {
            if let Some(ref ident) = field.ident {
                if ident == name {
                    return Some(callback(field));
                }
            }
        }
    }
    None
}