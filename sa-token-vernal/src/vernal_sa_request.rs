//! Vernal HTTP 请求快照到 Sa-Token 请求端口的适配对象。

use std::collections::HashMap;

use sa_token_adapter::{SaRequest, parse_cookies, parse_query_string};
use vernal_http::HttpRequestSnapshot;

/// 将 Vernal 拥有所有权的 HTTP 元数据快照适配为 `SaRequest`。
///
/// 该对象不借用 Web 框架原生 Request，因此可以安全跨越 `.await`，也不会复制
/// 请求 Body。Token 提取仍完全遵循 Sa-Token-Rust 的 Header、Cookie、Query
/// 优先级和校验语义。
pub struct VernalSaRequest<'request> {
    snapshot: &'request HttpRequestSnapshot,
}

impl<'request> VernalSaRequest<'request> {
    /// 包装 Vernal HTTP 请求快照。
    #[must_use]
    pub const fn new(snapshot: &'request HttpRequestSnapshot) -> Self {
        Self { snapshot }
    }

    fn cookies(&self) -> HashMap<String, String> {
        self.snapshot
            .headers()
            .get_all("cookie")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| parse_cookies(value).into_iter())
            .collect()
    }

    fn params(&self) -> HashMap<String, String> {
        self.snapshot
            .uri()
            .query()
            .map(parse_query_string)
            .unwrap_or_default()
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
        self.cookies().remove(name)
    }

    fn get_cookies(&self) -> HashMap<String, String> {
        self.cookies()
    }

    fn get_param(&self, name: &str) -> Option<String> {
        self.params().remove(name)
    }

    fn get_params(&self) -> HashMap<String, String> {
        self.params()
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
