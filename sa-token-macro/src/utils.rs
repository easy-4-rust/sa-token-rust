// Author: 金书记
//
//! 宏工具函数

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr, Token, punctuated::Punctuated};

/// 解析逗号分隔的字符串列表
#[allow(dead_code)]
pub fn parse_string_list(input: syn::parse::ParseStream) -> syn::Result<Vec<String>> {
    let vars = Punctuated::<LitStr, Token![,]>::parse_terminated(input)?;
    Ok(vars.into_iter().map(|lit| lit.value()).collect())
}

/// 为函数添加认证检查的包装代码
#[allow(dead_code)]
pub fn wrap_fn_with_auth_check(
    input: &ItemFn,
    check_type: &str,
    check_value: Option<&str>,
) -> TokenStream {
    let fn_name = &input.sig.ident;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_body = &input.block;
    let fn_attrs = &input.attrs;
    let fn_vis = &input.vis;
    let fn_asyncness = &input.sig.asyncness;
    let fn_generics = &input.sig.generics;

    let check_code = match check_type {
        "login" => quote! {
            // 检查登录状态
            // 这里会在中间件中实际执行，宏只是添加标记
        },
        "permission" => {
            let _perm = check_value.unwrap_or("");
            quote! {
                // 检查权限
                // 实际验证逻辑在中间件中执行
            }
        }
        "role" => {
            let _role_name = check_value.unwrap_or("");
            quote! {
                // 检查角色
                // 实际验证逻辑在中间件中执行
            }
        }
        _ => quote! {},
    };

    quote! {
        #(#fn_attrs)*
        #[allow(unused_variables)]
        #fn_vis #fn_asyncness fn #fn_name #fn_generics(#fn_inputs) #fn_output {
            #check_code
            #fn_body
        }
    }
}

/// 生成认证元数据属性
#[allow(dead_code)]
pub fn generate_auth_metadata(check_type: &str, value: Option<&str>) -> TokenStream {
    let metadata = if let Some(v) = value {
        format!("{}:{}", check_type, v)
    } else {
        check_type.to_string()
    };

    quote! {
        #[doc(hidden)]
        #[cfg_attr(feature = "sa-token-metadata", sa_token_auth_check = #metadata)]
    }
}
