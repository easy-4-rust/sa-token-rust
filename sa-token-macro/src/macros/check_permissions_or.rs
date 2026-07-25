// Author: 金书记
//
//! 多权限检查宏（OR逻辑）

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{ItemFn, LitStr, Token, parse::Parser, parse_macro_input};

/// 同时检查多个权限（OR逻辑）
///
/// 用户只需拥有任意一个指定的权限即可访问
///
/// # 示例
///
/// ```rust,ignore
/// #[sa_check_permissions_or("user:admin", "user:super")]
/// async fn manage_user() -> impl Responder {
///     // 拥有 user:admin 或 user:super 任一权限即可
///     "User managed"
/// }
/// ```
pub fn sa_check_permissions_or_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let parser = syn::punctuated::Punctuated::<LitStr, Token![,]>::parse_terminated;
    let permissions = parser.parse(attr).unwrap_or_default();
    let perm_lits: Vec<LitStr> = permissions.iter().cloned().collect();
    if perm_lits.is_empty() {
        return syn::Error::new(Span::call_site(), "At least one permission is required")
            .to_compile_error()
            .into();
    }
    let perm_values: Vec<String> = perm_lits.iter().map(|p| p.value()).collect();
    let perm_desc = LitStr::new(&perm_values.join(" | "), Span::call_site());

    let fn_name = &input.sig.ident;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_body = &input.block;
    let fn_attrs = &input.attrs;
    let fn_vis = &input.vis;
    let fn_asyncness = &input.sig.asyncness;
    let fn_generics = &input.sig.generics;
    let fn_where_clause = &input.sig.generics.where_clause;

    if fn_asyncness.is_none() {
        return syn::Error::new_spanned(fn_name, "Macro requires async function")
            .to_compile_error()
            .into();
    }

    let check_code = quote! {
        let __login_id = sa_token_core::StpUtil::get_login_id_as_string().await?;
        if !sa_token_core::StpUtil::has_permissions_or(&__login_id, &[#(#perm_lits),*]).await {
            return Err(sa_token_core::SaTokenError::PermissionDeniedDetail(String::from(#perm_desc)).into());
        }
    };

    let expanded: TokenStream2 = quote! {
        #(#fn_attrs)*
        #[doc(hidden)]
        #fn_vis #fn_asyncness fn #fn_name #fn_generics(#fn_inputs) #fn_output #fn_where_clause {
            #check_code
            #fn_body
        }
    };

    expanded.into()
}
