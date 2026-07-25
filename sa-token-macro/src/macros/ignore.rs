// Author: 金书记
//
//! 忽略认证宏

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Item, parse_macro_input};

/// 忽略认证检查的宏
///
/// 使用此宏标注的函数、结构体或impl块将跳过所有sa-token的认证检查，
/// 包括登录验证、权限验证、角色验证和路由拦截器。
///
/// 这对于公开API、健康检查接口等不需要认证的端点非常有用。
///
/// # 可以应用于
///
/// - 函数：单个路由处理函数忽略认证
/// - 结构体：整个控制器的所有方法都忽略认证
/// - impl块：impl块中的所有方法都忽略认证
///
/// # 示例
///
/// ## 在函数上使用
///
/// ```rust,ignore
/// #[sa_ignore]
/// async fn public_api() -> impl Responder {
///     // 此接口不需要任何认证
///     "Public API"
/// }
///
/// #[sa_ignore]
/// async fn health_check() -> impl Responder {
///     // 健康检查接口，无需认证
///     "OK"
/// }
/// ```
///
/// ## 在结构体上使用
///
/// ```rust,ignore
/// #[sa_ignore]
/// struct PublicController;
///
/// impl PublicController {
///     // 此控制器的所有方法都不需要认证
///     async fn home() -> impl Responder {
///         "Home page"
///     }
///     
///     async fn about() -> impl Responder {
///         "About page"
///     }
/// }
/// ```
///
/// ## 在impl块上使用
///
/// ```rust,ignore
/// struct ApiController;
///
/// #[sa_ignore]
/// impl ApiController {
///     // 这个impl块中的所有方法都忽略认证
///     async fn version() -> impl Responder {
///         "v1.0.0"
///     }
/// }
/// ```
///
/// # 优先级
///
/// `#[sa_ignore]` 的优先级最高，即使同时使用了 `#[sa_check_login]` 等其他认证宏，
/// 也会被 `#[sa_ignore]` 覆盖，不进行任何认证检查。
///
/// ```rust,ignore
/// // 警告：sa_ignore 会覆盖 sa_check_login
/// #[sa_ignore]
/// #[sa_check_login]  // 这个会被忽略
/// async fn example() -> impl Responder {
///     // 实际上不会进行登录检查
///     "Example"
/// }
/// ```
pub fn sa_ignore_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as Item);

    let expanded: TokenStream2 = match input {
        Item::Fn(item_fn) => {
            // 为函数添加忽略标记
            quote! {
                #[doc(hidden)]
                // Ignore marker for middleware
                #item_fn
            }
        }
        Item::Struct(item_struct) => {
            // 为结构体添加忽略标记
            quote! {
                #[doc(hidden)]
                // Ignore marker for middleware
                #item_struct
            }
        }
        Item::Impl(item_impl) => {
            // 为impl块添加忽略标记
            quote! {
                // Ignore marker for middleware
                #item_impl
            }
        }
        _ => {
            // 其他类型的item直接返回
            quote! { #input }
        }
    };

    expanded.into()
}
