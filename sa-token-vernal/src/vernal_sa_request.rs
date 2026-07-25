//! Vernal HTTP 请求快照到 Sa-Token 请求端口的适配对象。
//!
//! 增强版本：Cookie 和 Query 参数首次访问时解析，后续访问返回缓存结果。
//! 使用 `OnceLock` 实现线程安全的惰性初始化。

use std::{collections::HashMap, sync::OnceLock};

use sa_token_adapter::{SaRequest, parse_cookies, parse_query_string};
use vernal_http::HttpRequestSnapshot;

/// 将 Vernal 拥有所有权的 HTTP 元数据快照适配为 `SaRequest`。
///
/// 该对象不借用 Web 框架原生 Request，因此可以安全跨越 `.await`，也不会复制
/// 请求 Body。Token 提取仍完全遵循 Sa-Token-Rust 的 Header、Cookie、Query
/// 优先级和校验语义。
///
/// ## 缓存策略
///
/// Cookie 和 Query 参数在首次访问时解析并缓存到 `OnceLock`，后续访问直接返回
/// 缓存结果，避免重复解析开销。Header 不缓存（访问频率低且无解析开销）。
pub struct VernalSaRequest<'request> {
    /// Vernal HTTP 请求快照引用
    snapshot: &'request HttpRequestSnapshot,
    /// 缓存的 Cookie 解析结果（首次访问时惰性初始化）
    cookies: OnceLock<HashMap<String, String>>,
    /// 缓存的 Query 参数解析结果（首次访问时惰性初始化）
    params: OnceLock<HashMap<String, String>>,
}

impl<'request> VernalSaRequest<'request> {
    /// 包装 Vernal HTTP 请求快照。
    #[must_use]
    pub fn new(snapshot: &'request HttpRequestSnapshot) -> Self {
        Self {
            snapshot,
            cookies: OnceLock::new(),
            params: OnceLock::new(),
        }
    }

    /// 获取缓存的 Cookie 映射。首次调用时解析，后续返回缓存。
    fn cookies(&self) -> &HashMap<String, String> {
        self.cookies.get_or_init(|| {
            self.snapshot
                .headers()
                .get_all("cookie")
                .iter()
                .filter_map(|value| value.to_str().ok())
                .flat_map(|value| parse_cookies(value).into_iter())
                .collect()
        })
    }

    /// 获取缓存的 Query 参数映射。首次调用时解析，后续返回缓存。
    fn params(&self) -> &HashMap<String, String> {
        self.params.get_or_init(|| {
            self.snapshot
                .uri()
                .query()
                .map(parse_query_string)
                .unwrap_or_default()
        })
    }
}

impl SaRequest for VernalSaRequest<'_> {
    fn get_header(&self, name: &str) -> Option<String> {
        self.snapshot
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned)
    }

    fn get_headers(&self) -> HashMap<String, String> {
        self.snapshot
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.as_str().to_owned(), value.to_owned()))
            })
            .collect()
    }

    fn get_cookie(&self, name: &str) -> Option<String> {
        // 使用缓存的 Cookie 映射，避免重复解析
        self.cookies().get(name).cloned()
    }

    fn get_cookies(&self) -> HashMap<String, String> {
        // 返回缓存映射的克隆
        self.cookies().clone()
    }

    fn get_param(&self, name: &str) -> Option<String> {
        // 使用缓存的 Query 参数映射，避免重复解析
        self.params().get(name).cloned()
    }

    fn get_params(&self) -> HashMap<String, String> {
        // 返回缓存映射的克隆
        self.params().clone()
    }

    fn get_path(&self) -> String {
        self.snapshot.uri().path().to_owned()
    }

    fn get_method(&self) -> String {
        self.snapshot.method().as_str().to_owned()
    }

    fn get_uri(&self) -> String {
        self.snapshot.uri().to_string()
    }
}
