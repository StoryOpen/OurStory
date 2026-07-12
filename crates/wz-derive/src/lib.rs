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

/// Get a field-level re-root path from `#[wz(path = "...")]`.
/// Unlike `child` (which navigates one child of the current node), `path`
/// re-roots the whole field subtree: the field (and its nested children)
/// resolve from this path instead, until a deeper `path` re-roots again.
fn get_field_path(field: &syn::Field) -> Option<String> {
    for attr in &field.attrs {
        if attr.path().is_ident("wz") {
            if let Ok(list) = attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
                for meta in &list {
                    if let Meta::NameValue(nv) = meta {
                        if nv.path.is_ident("path") {
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

/// Resolve a child/sub-path node (R2): try an absolute lookup via the WZ
/// source first, then fall back to a relative `at_path` from the current node.
/// Emits a `Result<Node, WzError>` (uses `?`); the surrounding `node` variable
/// is the current resolution context.
fn wz_resolve(child: &str) -> proc_macro2::TokenStream {
    let c = syn::LitStr::new(child, proc_macro2::Span::call_site());
    quote! {
        match wz::source::default_source().node(#c) {
            Ok(__n) => __n,
            Err(_) => node.at_path(#c).map_err(|_| wz::WzError::NodeNotFound(#c.to_string()))?,
        }
    }
}

/// `Option` variant of `wz_resolve` (uses `.ok()` instead of `?`).
fn wz_resolve_opt(child: &str) -> proc_macro2::TokenStream {
    let c = syn::LitStr::new(child, proc_macro2::Span::call_site());
    quote! {
        match wz::source::default_source().node(#c) {
            Ok(__n) => Some(__n),
            Err(_) => node.at_path(#c).ok(),
        }
    }
}

/// Check if field has `#[wz(container_children)]` (deprecated; same as children without skip)
fn has_container_children(field: &syn::Field) -> bool {
    has_wz_attr(field, "container_children")
}

/// Check if field has `#[wz(skip)]`
fn has_skip(field: &syn::Field) -> bool {
    has_wz_attr(field, "skip")
}

/// Check if field has `#[wz(default)]` (flag only, or with a value)
fn has_default(field: &syn::Field) -> bool {
    has_wz_attr(field, "default")
}

/// Get custom default expression from `#[wz(default = expr)]`, or None.
fn get_default_value(field: &syn::Field) -> Option<syn::Expr> {
    for attr in &field.attrs {
        if attr.path().is_ident("wz") {
            if let Ok(list) = attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
                for meta in &list {
                    if meta.path().is_ident("default") {
                        if let Meta::NameValue(nv) = meta {
                            return Some(nv.value.clone());
                        }
                    }
                }
            }
        }
    }
    None
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

/// Per-element conversion for a collection whose element type is `inner`.
/// Scalars and `Vector2D` implement `TryFromNode` (they have no `wz_build`),
/// so they are converted with `TryFromNode`; everything else is assumed to be a
/// `WzAsset` and built with `wz_build`.
fn collection_elem_expr(inner: &Type) -> proc_macro2::TokenStream {
    if is_scalar(inner) || is_vector2d(inner) {
        quote! { <#inner as wz::TryFromNode<wz::Node>>::try_from_node(child) }
    } else {
        quote! { <#inner>::wz_build(&child, load_context, &sub_label) }
    }
}

/// Whether a collection element is built via `wz_build` (i.e. a `WzAsset`
/// struct) as opposed to `TryFromNode` (scalar / `Vector2D`). Controls
/// whether the `sub_label` binding is needed.
fn collection_is_struct(inner: &Type) -> bool {
    !(is_scalar(inner) || is_vector2d(inner))
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
/// Returns the field initializer expression (rhs of `field: <expr>`).
fn build_field_load(field: &syn::Field) -> proc_macro2::TokenStream {
    let field_name = field.ident.as_ref().unwrap();
    let name_str = wz_name(field);

    // Child name comes from `#[wz(child = "...")]` or `#[wz(path = "...")]`
    // (re-root), falling back to the field's snake/camel name.
    let explicit = get_child_attr(field).or_else(|| get_field_path(field));
    let child_name = explicit.clone().unwrap_or_else(|| name_str.clone());

    if has_skip(field) {
        return quote! { Default::default() };
    }

    let ty = &field.ty;

    // #[wz(image)] — Handle<Image>. Honors an explicit child/path to navigate to.
    if has_wz_attr(field, "image") {
        let nav = if let Some(c) = &explicit {
            let resolve = wz_resolve(c);
            quote! { let node = #resolve; }
        } else {
            quote! {}
        };
        let load_code = build_image_load_expr();
        return quote! { { #nav #load_code } };
    }

    // #[wz(origin)] — Vec2
    if has_wz_attr(field, "origin") {
        if let Some(dv) = get_default_value(field) {
            return quote! {
                {
                    node.try_get("origin")
                        .and_then(|n| n.read_origin(node).ok())
                        .map(|v| Vec2::new(v.x, v.y))
                        .unwrap_or(#dv)
                }
            };
        }
        return quote! {
            {
                let origin_node = node.try_get("origin").ok_or_else(|| {
                    wz::WzError::NodeNotFound(format!("{}/origin", node.path()))
                })?;
                let v = origin_node.read_origin(node)?;
                Vec2::new(v.x, v.y)
            }
        };
    }

    // Option<T>
    if let Some(inner) = is_option(ty) {
        if is_scalar(inner) || is_vector2d(inner) {
            return build_option_scalar(field_name, &child_name, inner);
        } else if is_handle_image(inner) {
            // Option<Handle<Image>> — try to load image, None if not present
            let image_load = build_image_load_expr();
            return quote! {
                {
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
        let base = if explicit.is_some() {
            let resolve = wz_resolve(child_name.as_str());
            quote! { let node = #resolve; }
        } else {
            quote! {}
        };
        return quote! {
            {
                #base
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
        let base = if explicit.is_some() {
            let resolve = wz_resolve(child_name.as_str());
            quote! { let node = #resolve; }
        } else {
            quote! {}
        };
        let children_tokens = if let Some(inner) = is_vec(ty) {
            build_children_vec(field_name, inner, &skip_list, numeric_only, require_child.as_deref())
        } else if let Some(inner) = is_hashmap_string(ty) {
            build_children_hashmap(field_name, inner, &skip_list, require_child.as_deref())
        } else {
            quote! { compile_error!("#[wz(children)] only supports Vec<T> or HashMap<String, T>"); }
        };
        return quote! { { #base #children_tokens } };
    }

    // Vec<T> — navigate to named child, then iterate
    if let Some(inner) = is_vec(ty) {
        return build_named_vec(field_name, &child_name, inner);
    }

    // HashMap<String, T> — navigate to named child, then iterate
    if let Some(inner) = is_hashmap_string(ty) {
        return build_named_hashmap(field_name, &child_name, inner);
    }

    // Scalar: i32, f32, String, bool, u32, u8
    if is_scalar(ty) {
        let default_val = get_default_value(field);
        return build_scalar_field(field_name, &child_name, ty, has_default(field), default_val.as_ref());
    }

    // Vector2D
    if is_vector2d(ty) {
        let default_val = get_default_value(field);
        return build_scalar_field(field_name, &child_name, ty, has_default(field), default_val.as_ref());
    }

    // Nested struct: T: WzChild
    let default_val = get_default_value(field);
    return build_nested_field(field_name, &child_name, has_default(field), default_val.as_ref());
}

fn build_scalar_field(
    _field_name: &syn::Ident,
    wz_child_name: &str,
    ty: &Type,
    use_default: bool,
    default_value: Option<&syn::Expr>,
) -> proc_macro2::TokenStream {
    let needs_cast = if let Type::Path(tp) = ty {
        let s = tp.path.get_ident().map(|i| i.to_string());
        matches!(s.as_deref(), Some("u32") | Some("u8"))
    } else { false };

    let resolve = wz_resolve(wz_child_name);
    let resolve_opt = wz_resolve_opt(wz_child_name);

    if use_default {
        let fallback = match default_value {
            Some(expr) => quote! { #expr },
            None => quote! { Default::default() },
        };
        if needs_cast {
            return quote! {
                {
                    #resolve_opt
                        .and_then(|n| {
                            let v: i32 = <i32 as wz::TryFromNode<wz::Node>>::try_from_node(n).ok()?;
                            Some(v as #ty)
                        })
                        .unwrap_or(#fallback)
                }
            };
        }
        return quote! {
            {
                #resolve_opt
                    .and_then(|n| <#ty as wz::TryFromNode<wz::Node>>::try_from_node(n).ok())
                    .unwrap_or(#fallback)
            }
        };
    }

    if needs_cast {
        return quote! {
            {
                let __child = #resolve;
                let v: i32 = <i32 as wz::TryFromNode<wz::Node>>::try_from_node(__child)?;
                v as #ty
            }
        };
    }

    quote! {
        {
            let __child = #resolve;
            <#ty as wz::TryFromNode<wz::Node>>::try_from_node(__child)?
        }
    }
}

fn build_option_scalar(
    _field_name: &syn::Ident,
    child_name: &str,
    inner: &Type,
) -> proc_macro2::TokenStream {
    let resolve_opt = wz_resolve_opt(child_name);
    quote! {
        #resolve_opt
            .and_then(|n| <#inner as wz::TryFromNode<wz::Node>>::try_from_node(n).ok())
    }
}

fn build_option_nested(
    _field_name: &syn::Ident,
    child_name: &str,
    inner: &Type,
) -> proc_macro2::TokenStream {
    let resolve_opt = wz_resolve_opt(child_name);
    quote! {
        #resolve_opt
            .and_then(|n| {
                let sub_label = format!("{}/{}", label_prefix, #child_name);
                #inner::wz_build(&n, load_context, &sub_label).ok()
            })
    }
}

fn build_nested_field(
    _field_name: &syn::Ident,
    child_name: &str,
    use_default: bool,
    default_value: Option<&syn::Expr>,
) -> proc_macro2::TokenStream {
    let resolve = wz_resolve(child_name);
    let resolve_opt = wz_resolve_opt(child_name);
    if use_default {
        let fallback = match default_value {
            Some(expr) => quote! { #expr },
            None => quote! { Default::default() },
        };
        return quote! {
            {
                #resolve_opt
                    .and_then(|n| {
                        let sub_label = format!("{}/{}", label_prefix, #child_name);
                        <_>::wz_build(&n, load_context, &sub_label).ok()
                    })
                    .unwrap_or(#fallback)
            }
        };
    }
    quote! {
        {
            let __child = #resolve;
            let sub_label = format!("{}/{}", label_prefix, #child_name);
            <_>::wz_build(&__child, load_context, &sub_label)?
        }
    }
}

fn build_named_vec(
    _field_name: &syn::Ident,
    child_name: &str,
    inner: &Type,
) -> proc_macro2::TokenStream {
    let sub_label = if collection_is_struct(inner) {
        quote! { let sub_label = format!("{}/{}/{}", label_prefix, #child_name, key_str); }
    } else {
        quote! {}
    };
    let elem = collection_elem_expr(inner);
    let parent = wz_resolve(child_name);
    quote! {
        {
            let parent = #parent;
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
                    #sub_label
                    #elem
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    }
}

/// Recursively count how many `HashMap<String, _>` wrappers are around `ty`.
fn hashmap_depth(ty: &Type) -> usize {
    if let Some(inner) = is_hashmap_string(ty) {
        1 + hashmap_depth(inner)
    } else {
        0
    }
}

/// Generate the inner body for nested `HashMap<String, ...>` fields.
/// At each HashMap level, emits a `for` loop + `HashMap::new()`.
/// At the leaf, emits `<T>::wz_build(...)?` whose `?` propagates through
/// all enclosing statements to the outermost `.map()` closure.
///
/// `child_var` is the expression for the node whose children we iterate.
fn build_hashmap_nest(
    inner: &Type,
    child_var: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    if let Some(sub_inner) = is_hashmap_string(inner) {
        let inner_block = build_hashmap_nest(sub_inner, &quote! { inner_child });
        quote! {
            {
                let mut map = std::collections::HashMap::new();
                for (inner_key, inner_child) in #child_var.children() {
                    let inner_key_str = inner_key.to_string();
                    let val = #inner_block;
                    map.insert(inner_key_str, val);
                }
                map
            }
        }
    } else {
        quote! { <#inner>::wz_build(&#child_var, load_context, &sub_label)? }
    }
}

fn build_named_hashmap(
    _field_name: &syn::Ident,
    child_name: &str,
    inner: &Type,
) -> proc_macro2::TokenStream {
    let parent = wz_resolve(child_name);
    let depth = hashmap_depth(inner);

    if depth > 0 {
        // Nested HashMap<String, HashMap<String, ..., T>>.
        // Generate a chain of `for` loops inside the `.map()` closure.
        let inner_body = build_hashmap_nest(inner, &quote! { child });
        let sub_label = if collection_is_struct(inner) {
            quote! { let sub_label = format!("{}/{}/{}", label_prefix, #child_name, key_str); }
        } else {
            quote! {}
        };
        return quote! {
            {
                let parent = #parent;
                parent.children()
                    .into_iter()
                    .map(|(key, child)| {
                        let key_str = key.to_string();
                        #sub_label
                        Ok((key_str, #inner_body))
                    })
                    .collect::<Result<_, wz::WzError>>()?
            }
        };
    }

    let sub_label = if collection_is_struct(inner) {
        quote! { let sub_label = format!("{}/{}/{}", label_prefix, #child_name, key_str); }
    } else {
        quote! {}
    };
    let elem = collection_elem_expr(inner);
    quote! {
        {
            let parent = #parent;
            parent.children()
                .into_iter()
                .map(|(key, child)| {
                    let key_str = key.to_string();
                    #sub_label
                    let val = #elem?;
                    Ok((key_str, val))
                })
                .collect::<Result<_, wz::WzError>>()?
        }
    }
}

fn build_children_vec(
    _field_name: &syn::Ident,
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
    let sub_label = if collection_is_struct(inner) {
        quote! { let sub_label = format!("{}/{}", label_prefix, key_str); }
    } else {
        quote! {}
    };
    let elem = collection_elem_expr(inner);

    let body = quote! {
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
                #sub_label
                #elem
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    if numeric_only {
        return quote! { { #body } };
    }

    quote! { { #body } }
}

fn build_children_hashmap(
    _field_name: &syn::Ident,
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
    let sub_label = if collection_is_struct(inner) {
        quote! { let sub_label = format!("{}/{}", label_prefix, key_str); }
    } else {
        quote! {}
    };
    let elem = collection_elem_expr(inner);

    let body = quote! {
        let skip_names: &[&str] = &[#(#skip_elems),*];
        node.children()
            .into_iter()
            .filter_map(|(key, child)| {
                let key_str = key.to_string();
                if skip_names.contains(&key_str.as_str()) { return None; }
                #require_check
                #sub_label
                match #elem {
                    Ok(val) => Some((key_str, val)),
                    Err(e) => {
                        bevy::log::warn!("skipping child '{}': {}", key_str, e);
                        None
                    }
                }
            })
            .collect()
    };

    quote! { { #body } }
}

/// Generate the full WzAsset trait impl (CONST, path method, wz_build)
fn generate_wz_asset_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
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

    let ext = struct_attr_value(input, "ext").unwrap_or_else(|| {
        // Fall back to snake_case of the struct name without "Asset" suffix
        let s = name.to_string();
        s.strip_suffix("Asset").unwrap_or(&s).to_lowercase()
    });
    let path_template = struct_attr_value(input, "path").unwrap_or_else(|| ".".to_string());

    // Generate field initializers for wz_build
    let mut field_inits = Vec::new();
    for field in fields {
        let ident = field.ident.as_ref().unwrap();
        let init = build_field_load(field);
        field_inits.push(quote! { #ident: #init });
    }

    // Generate path conversion logic
    let path_logic = build_path_method(&path_template, &ext);

    quote! {
        impl crate::wz::WzAsset for #name {
            const EXTENSION: &'static str = #ext;

            fn asset_path_to_wz_path(asset_path: &str) -> String {
                #path_logic
            }

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

/// Build the path conversion method body.
fn build_path_method(template: &str, ext: &str) -> proc_macro2::TokenStream {
    if template == "." {
        // Just strip prefix and extension
        return quote! {
            {
                let s = asset_path
                    .strip_prefix("wz://").unwrap_or(asset_path)
                    .strip_suffix(concat!(".", #ext)).unwrap_or(asset_path);
                s.to_string()
            }
        };
    }

    if template.contains("{id}") {
        return quote! {
            {
                let s = asset_path
                    .strip_prefix("wz://").unwrap_or(asset_path)
                    .strip_suffix(concat!(".", #ext)).unwrap_or(asset_path);
                let id = s.trim_end_matches(".img").rsplit('/').next()
                    .and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
                format!(#template, id = id)
            }
        };
    }

    if template.contains("{leaf}") {
        // Special: split stripped path at last '/', dir+leaf
        return quote! {
            {
                let s = asset_path
                    .strip_prefix("wz://").unwrap_or(asset_path)
                    .strip_suffix(concat!(".", #ext)).unwrap_or(asset_path);
                let (dir, leaf) = s.rsplit_once('/').unwrap_or(("", s));
                format!(#template, dir = dir, leaf = leaf)
            }
        };
    }

    // Literal path — ignore asset_path entirely
    quote! { #template.to_string() }
}

/// Generate the complete output for the derive macro
fn wz_asset_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    generate_wz_asset_impl(input)
}

#[proc_macro_derive(WzAsset, attributes(wz))]
pub fn derive_wz_asset(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    wz_asset_impl(&input).into()
}
