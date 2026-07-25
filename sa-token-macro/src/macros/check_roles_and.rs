// Author: 金书记
//
//! 多角色检查宏（AND逻辑）

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{ItemFn, LitStr, Token, parse::Parser, parse_macro_input};

/// 同时检查多个角色（AND逻辑）
///
/// 用户必须拥有所有指定的角色才能访问
///
/// # 示例
///
/// ```rust,ignore
/// #[sa_check_roles_and("admin", "super")]
/// async fn super_admin_panel() -> impl Responder {
///     // 需要同时拥有 admin 和 super 角色
///     "Super admin panel"
/// }
/// ```
pub fn sa_check_roles_and_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let parser = syn::punctuated::Punctuated::<LitStr, Token![,]>::parse_terminated;
    let roles = parser.parse(attr).unwrap_or_default();
    let role_lits: Vec<LitStr> = roles.iter().cloned().collect();
    if role_lits.is_empty() {
        return syn::Error::new(Span::call_site(), "At least one role is required")
            .to_compile_error()
            .into();
    }
    // 生成每个角色的检查代码，不需要存储角色值列表

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
        #(sa_token_core::StpUtil::check_role(&__login_id, #role_lits).await?;)*
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
