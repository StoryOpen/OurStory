use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, DeriveInput, Data, Fields, Type, TypePath, PathArguments,
    GenericArgument, Meta, Expr, Lit,
    punctuated::Punctuated,
    token::Comma,
};

/// Convert snake_case to camelCase.
/// "max_hp" → "maxHP", "body_attack" → "bodyAttack", "PADamage" → "PADamage" (no change)
fn snake_to_camel(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut next_upper = false;

    while let Some(c) = chars.next() {
        if c == '_' {
            next_upper = true;
        } else if next_upper {
            result.push(c.to_ascii_uppercase());
            next_upper = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Extract the WZ child name for a field: explicit `#[wz(rename = "...")]` or snake→camel.
fn wz_name(field: &syn::Field) -> String {
    for attr in &field.attrs {
        if attr.path().is_ident("wz") {
            if let Ok(list) = attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
                for meta in &list {
                    if let Meta::NameValue(nv) = meta {
                        if nv.path.is_ident("rename") {
                            if let Expr::Lit(expr_lit) = &nv.value {
                                if let Lit::Str(s) = &expr_lit.lit {
                                    return s.value();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    snake_to_camel(&field.ident.as_ref().map(|i| i.to_string()).unwrap_or_default())
}

/// Check if a field has a specific wz attribute (flag, no value).
fn has_wz_attr(field: &syn::Field, attr_name: &str) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("wz") {
            if let Ok(list) = attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
                for meta in &list {
                    if meta.path().is_ident(attr_name) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Get children skip list from `#[wz(children(skip = ["a", "b"]))]`
fn get_children_skip(field: &syn::Field) -> Vec<String> {
    for attr in &field.attrs {
        if attr.path().is_ident("wz") {
            if let Ok(list) = attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
                for meta in &list {
                    if meta.path().is_ident("children") {
                        if let Meta::List(children_meta) = meta {
                            let Ok(nested) = children_meta.parse_args_with(
                                Punctuated::<Meta, Comma>::parse_terminated
                            ) else { continue };
                            for sub in &nested {
                                if sub.path().is_ident("skip") {
                                    if let Meta::NameValue(nv) = sub {
                                        if let Expr::Array(arr) = &nv.value {
                                            return arr.elems.iter().filter_map(|e| {
                                                if let Expr::Lit(el) = e {
                                                    if let Lit::Str(s) = &el.lit {
                                                        return Some(s.value());
                                                    }
                                                }
                                                None
                                            }).collect();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Vec::new()
}

/// Check if `#[wz(children(numeric_only))]` is present
fn has_children_numeric_only(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("wz") {
            if let Ok(list) = attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
                for meta in &list {
                    if meta.path().is_ident("children") {
                        if let Meta::List(children_meta) = meta {
                            let Ok(nested) = children_meta.parse_args_with(
                                Punctuated::<Meta, Comma>::parse_terminated
                            ) else { continue };
                            for sub in &nested {
                                if sub.path().is_ident("numeric_only") {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Check `#[wz(children(require_child = "0"))]`
fn get_children_require_child(field: &syn::Field) -> Option<String> {
    for attr in &field.attrs {
        if attr.path().is_ident("wz") {
            if let Ok(list) = attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
                for meta in &list {
                    if meta.path().is_ident("children") {
                        if let Meta::List(children_meta) = meta {
                            let Ok(nested) = children_meta.parse_args_with(
                                Punctuated::<Meta, Comma>::parse_terminated
                            ) else { continue };
                            for sub in &nested {
                                if sub.path().is_ident("require_child") {
                                    if let Meta::NameValue(nv) = sub {
                                        if let Expr::Lit(el) = &nv.value {
                                            if let Lit::Str(s) = &el.lit {
                                                return Some(s.value());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Get explicit child name from `#[wz(child = "info")]`
fn get_child_attr(field: &syn::Field) -> Option<String> {
    for attr in &field.attrs {
        if attr.path().is_ident("wz") {
            if let Ok(list) = attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
                for meta in &list {
                    if meta.path().is_ident("child") {
                        if let Meta::NameValue(nv) = meta {
                            if let Expr::Lit(el) = &nv.value {
                                if let Lit::Str(s) = &el.lit {
                                    return Some(s.value());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Check if field has `#[wz(container_children)]` (deprecated; same as children without skip)
fn has_container_children(field: &syn::Field) -> bool {
    has_wz_attr(field, "container_children")
}

/// Check if field has `#[wz(skip)]`
fn has_skip(field: &syn::Field) -> bool {
    has_wz_attr(field, "skip")
}

/// Check if field has `#[wz(default)]`
fn has_default(field: &syn::Field) -> bool {
    has_wz_attr(field, "default")
}

/// Check if type is `Handle<Image>`
fn is_handle_image(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        let segs = &path.segments;
        if segs.len() == 1 {
            let seg = &segs[0];
            if seg.ident == "Handle" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    for arg in &args.args {
                        if let GenericArgument::Type(Type::Path(tp)) = arg {
                            if tp.path.is_ident("Image") {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Build an inline expression to load an image from a WZ node.
/// Evaluates to `Result<Handle<Image>, wz::WzError>`.
/// Build an inline expression to load an image from a WZ node into Handle<Image>.
/// Requires `load_context`, `node`, and `label_prefix` in scope.
/// The expression uses `?` internally, so the enclosing function must return Result.
fn build_image_load_expr() -> proc_macro2::TokenStream {
    quote! {
        {
            let label = format!("{}/{}", label_prefix, node.name());
            if load_context.has_labeled_asset(&label) {
                load_context.get_label_handle::<bevy::prelude::Image>(&label)
            } else {
                let dyn_img = node.extract_image().map_err(|_| {
                    wz::WzError::ValueError(format!("failed to extract image at {}", node.path()))
                })?;
                let rgba = dyn_img.to_rgba8();
                let (width, height) = rgba.dimensions();
                let image = bevy::prelude::Image::new(
                    bevy::render::render_resource::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    rgba.into_raw(),
                    bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                    RenderAssetUsages::MAIN_WORLD |
                        RenderAssetUsages::RENDER_WORLD,
                );
                let owned_label: String = label;
                load_context.add_labeled_asset(owned_label, image)
            }
        }
    }
}

/// Check if type is `Vec2`
#[allow(dead_code)]
fn is_vec2(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        path.is_ident("Vec2")
    } else {
        false
    }
}

/// Check if type is `Vec<T>`
fn is_vec(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        let segs = &path.segments;
        if segs.len() == 1 && segs[0].ident == "Vec" {
            if let PathArguments::AngleBracketed(args) = &segs[0].arguments {
                if let Some(GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner);
                }
            }
        }
    }
    None
}

/// Check if type is `HashMap<String, T>`
fn is_hashmap_string(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        let segs = &path.segments;
        if segs.len() >= 1 {
            let last = &segs[segs.len() - 1];
            if last.ident == "HashMap" {
                if let PathArguments::AngleBracketed(args) = &last.arguments {
                    let mut args_iter = args.args.iter();
                    // Check first arg is String
                    match args_iter.next() {
                        Some(GenericArgument::Type(Type::Path(tp))) if tp.path.is_ident("String") => {},
                        _ => return None,
                    }
                    if let Some(GenericArgument::Type(inner)) = args_iter.next() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

/// Check if type is Option<T>
fn is_option(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        let segs = &path.segments;
        if segs.len() == 1 && segs[0].ident == "Option" {
            if let PathArguments::AngleBracketed(args) = &segs[0].arguments {
                if let Some(GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner);
                }
            }
        }
    }
    None
}

/// Check if type is a known scalar (i32, f32, String, bool, u32, u8)
fn is_scalar(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        let s = path.get_ident().map(|i| i.to_string());
        matches!(s.as_deref(), Some("i32") | Some("u32") | Some("f32") | Some("String") | Some("bool") | Some("u8"))
    } else {
        false
    }
}

/// Check if type is a known scalar that can use TryFromNode directly
#[allow(dead_code)]
fn is_try_from_scalar(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        let s = path.get_ident().map(|i| i.to_string());
        matches!(s.as_deref(), Some("i32") | Some("f32") | Some("String") | Some("bool"))
    } else {
        false
    }
}

/// Check if type is Vector2D (from wz crate)
fn is_vector2d(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        path.is_ident("Vector2D")
    } else {
        false
    }
}

/// Get the struct-level attribute value
fn struct_attr_value(input: &DeriveInput, key: &str) -> Option<String> {
    for attr in &input.attrs {
        if attr.path().is_ident("wz") {
            if let Ok(list) = attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
                for meta in &list {
                    match meta {
                        Meta::NameValue(nv) if nv.path.is_ident(key) => {
                            if let Expr::Lit(el) = &nv.value {
                                if let Lit::Str(s) = &el.lit {
                                    return Some(s.value());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    None
}

/// Build the wz_build code for a single field.
/// Returns (field_init_code) — the expression to assign to this field.
fn build_field_load(field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = field.ident.as_ref().unwrap();
    let name_str = wz_name(field);

    if has_skip(field) {
        return quote! { #field_name: Default::default() };
    }

    let ty = &field.ty;

    // #[wz(image)] — Handle<Image>
    if has_wz_attr(field, "image") {
        let load_code = build_image_load_expr();
        return quote! { #field_name: #load_code };
    }

    // #[wz(origin)] — Vec2
    if has_wz_attr(field, "origin") {
        return quote! {
            #field_name: {
                let origin_node = node.try_get("origin").ok_or_else(|| {
                    wz::WzError::NodeNotFound(format!("{}/origin", node.path()))
                })?;
                let v = origin_node.read_origin(node)?;
                Vec2::new(v.0, v.1)
            }
        };
    }

    // Option<T>
    if let Some(inner) = is_option(ty) {
        let child_name = get_child_attr(field).unwrap_or_else(|| name_str.clone());
        if is_scalar(inner) || is_vector2d(inner) {
            return build_option_scalar(field_name, &child_name, inner);
        } else if is_handle_image(inner) {
            // Option<Handle<Image>> — try to load image, None if not present
            let image_load = build_image_load_expr();
            return quote! {
                #field_name: {
                    if node.has_image_data() {
                        Some(#image_load)
                    } else {
                        None
                    }
                }
            };
        } else {
            // Option<SomeStruct> — try to load child, None if missing
            return build_option_nested(field_name, &child_name, inner);
        }
    }

    // #[wz(children_images)] — Vec<Handle<Image>>, each child is a PNG node
    if has_wz_attr(field, "children_images") {
        let load_expr = build_image_load_expr();
        return quote! {
            #field_name: {
                let mut items: Vec<_> = node.children()
                    .into_iter()
                    .filter_map(|(key, child)| {
                        let key_str = key.to_string();
                        key_str.parse::<u32>().ok().map(|idx| (idx, child, key_str))
                    })
                    .collect();
                items.sort_by_key(|(idx, _, _)| *idx);
                items.into_iter()
                    .map(|(_idx, child, key_str)| {
                        let prefix = format!("{}/{}", label_prefix, key_str);
                        let node = &child;
                        let label_prefix = prefix.as_str();
                        (|| -> Result<_, wz::WzError> { Ok(#load_expr) })()
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
        };
    }

    // #[wz(children)] or #[wz(container_children)] — iterate current node's children
    if has_wz_attr(field, "children") || has_container_children(field) {
        let skip_list = get_children_skip(field);
        let numeric_only = has_children_numeric_only(field);
        let require_child = get_children_require_child(field);
        let skip_fields: Vec<String> = {
            // Also skip fields that are already defined on this struct (like "delay")
            // We can't easily get sibling field names here; rely on explicit skip list
            skip_list
        };

        if let Some(inner) = is_vec(ty) {
            return build_children_vec(field_name, inner, &skip_fields, numeric_only, require_child.as_deref());
        }
        if let Some(inner) = is_hashmap_string(ty) {
            return build_children_hashmap(field_name, inner, &skip_fields, require_child.as_deref());
        }
        return quote! { compile_error!("#[wz(children)] only supports Vec<T> or HashMap<String, T>"); };
    }

    // Vec<T> — navigate to named child, then iterate
    if let Some(inner) = is_vec(ty) {
        let child_name = get_child_attr(field).unwrap_or_else(|| name_str.clone());
        return build_named_vec(field_name, &child_name, inner);
    }

    // HashMap<String, T> — navigate to named child, then iterate
    if let Some(inner) = is_hashmap_string(ty) {
        let child_name = get_child_attr(field).unwrap_or_else(|| name_str.clone());
        return build_named_hashmap(field_name, &child_name, inner);
    }

    // Scalar: i32, f32, String, bool, u32, u8
    if is_scalar(ty) {
        return build_scalar_field(field_name, &name_str, ty, has_default(field));
    }

    // Vector2D
    if is_vector2d(ty) {
        return build_scalar_field(field_name, &name_str, ty, has_default(field));
    }

    // Nested struct: T: WzChild
    let child_name = get_child_attr(field).unwrap_or_else(|| name_str.clone());
    return build_nested_field(field_name, &child_name, has_default(field));
}

fn build_scalar_field(
    field_name: &syn::Ident,
    wz_child_name: &str,
    ty: &Type,
    use_default: bool,
) -> proc_macro2::TokenStream {
    // Check if this is a type that needs casting from i32 (u32, u8)
    let needs_cast = if let Type::Path(tp) = ty {
        let s = tp.path.get_ident().map(|i| i.to_string());
        matches!(s.as_deref(), Some("u32") | Some("u8"))
    } else { false };

    if use_default {
        if needs_cast {
            return quote! {
                #field_name: {
                    node.at_path(#wz_child_name)
                        .ok()
                        .and_then(|n| {
                            let v: i32 = <i32 as wz::TryFromNode<wz::Node>>::try_from_node(n).ok()?;
                            Some(v as #ty)
                        })
                        .unwrap_or_default()
                }
            };
        }
        return quote! {
            #field_name: {
                node.at_path(#wz_child_name)
                    .ok()
                    .and_then(|n| {
                        <#ty as wz::TryFromNode<wz::Node>>::try_from_node(n).ok()
                    })
                    .unwrap_or_default()
            }
        };
    }

    if needs_cast {
        return quote! {
            #field_name: {
                let child = node.at_path(#wz_child_name)?;
                let v: i32 = <i32 as wz::TryFromNode<wz::Node>>::try_from_node(child)?;
                v as #ty
            }
        };
    }

    quote! {
        #field_name: {
            let child = node.at_path(#wz_child_name)?;
            <#ty as wz::TryFromNode<wz::Node>>::try_from_node(child)?
        }
    }
}

fn build_option_scalar(
    field_name: &syn::Ident,
    child_name: &str,
    inner: &Type,
) -> proc_macro2::TokenStream {
    quote! {
        #field_name: node.at_path(#child_name)
            .ok()
            .and_then(|n| <#inner as wz::TryFromNode<wz::Node>>::try_from_node(n).ok())
    }
}

fn build_option_nested(
    field_name: &syn::Ident,
    child_name: &str,
    inner: &Type,
) -> proc_macro2::TokenStream {
    quote! {
        #field_name: node.at_path(#child_name)
            .ok()
            .and_then(|n| {
                let sub_label = format!("{}/{}", label_prefix, #child_name);
                #inner::wz_build(&n, load_context, &sub_label).ok()
            })
    }
}

fn build_nested_field(
    field_name: &syn::Ident,
    child_name: &str,
    use_default: bool,
) -> proc_macro2::TokenStream {
    if use_default {
        return quote! {
            #field_name: {
                node.at_path(#child_name)
                    .ok()
                    .and_then(|n| {
                        let sub_label = format!("{}/{}", label_prefix, #child_name);
                        <_>::wz_build(&n, load_context, &sub_label).ok()
                    })
                    .unwrap_or_default()
            }
        };
    }
    quote! {
        #field_name: {
            let child = node.at_path(#child_name)
                .map_err(|_| wz::WzError::NodeNotFound(
                    format!("{}/{}", node.path(), #child_name)
                ))?;
            let sub_label = format!("{}/{}", label_prefix, #child_name);
            <_>::wz_build(&child, load_context, &sub_label)?
        }
    }
}

fn build_named_vec(
    field_name: &syn::Ident,
    child_name: &str,
    inner: &Type,
) -> proc_macro2::TokenStream {
    quote! {
        #field_name: {
            let parent = node.at_path(#child_name)?;
            let mut items: Vec<_> = parent
                .children()
                .into_iter()
                .filter_map(|(key, child)| {
                    let key_str = key.to_string();
                    key_str.parse::<u32>().ok().map(|idx| (idx, child, key_str))
                })
                .collect();
            items.sort_by_key(|(idx, _, _)| *idx);
            items.into_iter()
                .map(|(idx, child, key_str)| {
                    let sub_label = format!("{}/{}/{}", label_prefix, #child_name, key_str);
                    <#inner>::wz_build(&child, load_context, &sub_label)
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    }
}

fn build_named_hashmap(
    field_name: &syn::Ident,
    child_name: &str,
    inner: &Type,
) -> proc_macro2::TokenStream {
    quote! {
        #field_name: {
            let parent = node.at_path(#child_name)?;
            parent.children()
                .into_iter()
                .map(|(key, child)| {
                    let key_str = key.to_string();
                    let sub_label = format!("{}/{}/{}", label_prefix, #child_name, key_str);
                    let val = <#inner>::wz_build(&child, load_context, &sub_label)?;
                    Ok((key_str, val))
                })
                .collect::<Result<_, wz::WzError>>()?
        }
    }
}

fn build_children_vec(
    field_name: &syn::Ident,
    inner: &Type,
    skip_list: &[String],
    numeric_only: bool,
    require_child: Option<&str>,
) -> proc_macro2::TokenStream {
    // Build skip set as a static array, use contains check in closure
    let skip_elems: Vec<&str> = skip_list.iter().map(|s| s.as_str()).collect();
    let require_check = if let Some(rc) = require_child {
        quote! {
            if !child.try_get(#rc).is_some() { return None; }
        }
    } else {
        quote! {}
    };

    if numeric_only {
        return quote! {
            #field_name: {
                let skip_names: &[&str] = &[#(#skip_elems),*];
                let mut items: Vec<_> = node.children()
                    .into_iter()
                    .filter_map(|(key, child)| {
                        let key_str = key.to_string();
                        if skip_names.contains(&key_str.as_str()) { return None; }
                        #require_check
                        key_str.parse::<u32>().ok().map(|idx| (idx, child, key_str))
                    })
                    .collect();
                items.sort_by_key(|(idx, _, _)| *idx);
                items.into_iter()
                    .map(|(_idx, child, key_str)| {
                        let sub_label = format!("{}/{}", label_prefix, key_str);
                        <#inner>::wz_build(&child, load_context, &sub_label)
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
        };
    }

    quote! {
        #field_name: {
            let skip_names: &[&str] = &[#(#skip_elems),*];
            let mut items: Vec<_> = node.children()
                .into_iter()
                .filter_map(|(key, child)| {
                    let key_str = key.to_string();
                    if skip_names.contains(&key_str.as_str()) { return None; }
                    #require_check
                    key_str.parse::<u32>().ok().map(|idx| (idx, child, key_str))
                })
                .collect();
            items.sort_by_key(|(idx, _, _)| *idx);
            items.into_iter()
                .map(|(_idx, child, key_str)| {
                    let sub_label = format!("{}/{}", label_prefix, key_str);
                    <#inner>::wz_build(&child, load_context, &sub_label)
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    }
}

fn build_children_hashmap(
    field_name: &syn::Ident,
    inner: &Type,
    skip_list: &[String],
    require_child: Option<&str>,
) -> proc_macro2::TokenStream {
    let skip_elems: Vec<&str> = skip_list.iter().map(|s| s.as_str()).collect();
    let require_check = if let Some(rc) = require_child {
        quote! {
            if !child.try_get(#rc).is_some() { return None; }
        }
    } else {
        quote! {}
    };

    quote! {
        #field_name: {
            let skip_names: &[&str] = &[#(#skip_elems),*];
            node.children()
                .into_iter()
                .filter_map(|(key, child)| {
                    let key_str = key.to_string();
                    if skip_names.contains(&key_str.as_str()) { return None; }
                    #require_check
                    let sub_label = format!("{}/{}", label_prefix, key_str);
                    match <#inner>::wz_build(&child, load_context, &sub_label) {
                        Ok(val) => Some((key_str, val)),
                        Err(e) => {
                            bevy::log::warn!("skipping child '{}': {}", key_str, e);
                            None
                        }
                    }
                })
                .collect()
        }
    }
}

/// Generate the wz_build function for a struct
fn generate_wz_child_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return quote! { compile_error!("WzAsset only supports named fields"); };
            }
        },
        _ => {
            return quote! { compile_error!("WzAsset only supports structs"); };
        }
    };

    let mut field_inits = Vec::new();

    for field in fields {
        let init = build_field_load(field);
        field_inits.push(init);
    }

    quote! {
        impl crate::wz::WzAsset for #name {
            fn wz_build(
                node: &wz::Node,
                load_context: &mut bevy::asset::LoadContext<'_>,
                label_prefix: &str,
            ) -> Result<Self, wz::WzError> {
                Ok(#name {
                    #(#field_inits,)*
                })
            }
        }
    }
}

/// Generate an AssetLoader struct and impl for the top-level type
fn generate_asset_loader(input: &DeriveInput, ext: &str, path_template: &str) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let loader_name = format_ident!("{}Loader", name);
    let error_name = format_ident!("{}LoaderError", name);

    // Build path template logic
    let path_logic = build_path_template(path_template);

    quote! {
        #[derive(Debug, thiserror::Error)]
        pub enum #error_name {
            #[error("WZ error: {0}")]
            Wz(#[from] wz::WzError),
            #[error("WZ source error: {0}")]
            Source(#[from] wz::source::WzSourceError),
            #[error("IO error: {0}")]
            Io(#[from] std::io::Error),
        }

        #[derive(Default, bevy::prelude::TypePath)]
        pub struct #loader_name;

        impl bevy::asset::AssetLoader for #loader_name {
            type Asset = #name;
            type Settings = ();
            type Error = #error_name;

            async fn load(
                &self,
                _reader: &mut dyn bevy::asset::io::Reader,
                _settings: &(),
                load_context: &mut bevy::asset::LoadContext<'_>,
            ) -> Result<Self::Asset, Self::Error> {
                let asset_path = load_context.path().path().to_string_lossy().to_string();
                let stripped = asset_path
                    .strip_prefix("wz://")
                    .unwrap_or(&asset_path);
                let stripped = stripped
                    .strip_suffix(concat!(".", #ext))
                    .unwrap_or(stripped);

                let wz_path = #path_logic;

                let source = wz::source::default_source();
                let node = source.node(&wz_path).await?;

                Ok(crate::wz::WzAsset::wz_build(&node, load_context, "")?)
            }

            fn extensions(&self) -> &[&str] {
                &[#ext]
            }
        }
    }
}

/// Build code that constructs the WZ path from the asset path.
/// Supports `{id}` placeholder extracted from the asset path.
fn build_path_template(template: &str) -> proc_macro2::TokenStream {
    if template.contains("{id}") {
        quote! {
            {
                let id_str = stripped
                    .trim_end_matches(".img")
                    .rsplit('/')
                    .next()
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);
                format!(#template, id = id_str)
            }
        }
    } else {
        quote! { stripped.to_string() }
    }
}

/// Generate the complete output for the derive macro
fn wz_asset_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let child_impl = generate_wz_child_impl(input);

    let ext = struct_attr_value(input, "asset_ext");
    let path_template = struct_attr_value(input, "path_template");

    if let (Some(ext), Some(path_template)) = (&ext, &path_template) {
        let loader_impl = generate_asset_loader(input, ext, path_template);
        quote! {
            #child_impl
            #loader_impl
        }
    } else {
        child_impl
    }
}

#[proc_macro_derive(WzAsset, attributes(wz))]
pub fn derive_wz_asset(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    wz_asset_impl(&input).into()
}
