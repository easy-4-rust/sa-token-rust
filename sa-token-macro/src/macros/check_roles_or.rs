// Author: 金书记
//
//! 多角色检查宏（OR逻辑）

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{ItemFn, LitStr, Token, parse::Parser, parse_macro_input};

/// 同时检查多个角色（OR逻辑）
///
/// 用户只需拥有任意一个指定的角色即可访问
///
/// # 示例
///
/// ```rust,ignore
/// #[sa_check_roles_or("admin", "moderator")]
/// async fn moderate_content() -> impl Responder {
///     // 拥有 admin 或 moderator 任一角色即可
///     "Content moderated"
/// }
/// ```
pub fn sa_check_roles_or_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let parser = syn::punctuated::Punctuated::<LitStr, Token![,]>::parse_terminated;
    let roles = parser.parse(attr).unwrap_or_default();
    let role_lits: Vec<LitStr> = roles.iter().cloned().collect();
    if role_lits.is_empty() {
        return syn::Error::new(Span::call_site(), "At least one role is required")
            .to_compile_error()
            .into();
    }
    let role_values: Vec<String> = role_lits.iter().map(|r| r.value()).collect();
    let role_desc = LitStr::new(&role_values.join(" | "), Span::call_site());

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
        let mut __has_role = false;
        #(
            if sa_token_core::StpUtil::has_role(&__login_id, #role_lits).await {
                __has_role = true;
            }
        )*
        if !__has_role {
            return Err(sa_token_core::SaTokenError::RoleDenied(String::from(#role_desc)).into());
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
