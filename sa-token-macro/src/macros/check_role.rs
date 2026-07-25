// Author: 金书记
//
//! 角色检查宏

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

/// 检查角色的宏
///
/// 使用此宏标注的函数会在执行前检查用户是否拥有指定角色
///
/// # 参数
///
/// - `role` - 角色名称，如 "admin"、"user"、"vip"
///
/// # 示例
///
/// ```rust,ignore
/// #[sa_check_role("admin")]
/// async fn admin_panel() -> impl Responder {
///     // 只有 admin 角色才能访问
///     "Admin panel"
/// }
/// ```
pub fn sa_check_role_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let role = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_body = &input.block;
    let fn_attrs = &input.attrs;
    let fn_vis = &input.vis;
    let fn_asyncness = &input.sig.asyncness;
    let fn_generics = &input.sig.generics;
    let fn_where_clause = &input.sig.generics.where_clause;
    let role_value = role.value();

    if fn_asyncness.is_none() {
        return syn::Error::new_spanned(fn_name, "Macro requires async function")
            .to_compile_error()
            .into();
    }

    let check_code = quote! {
        let __login_id = sa_token_core::StpUtil::get_login_id_as_string().await?;
        sa_token_core::StpUtil::check_role(&__login_id, #role_value).await?;
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
