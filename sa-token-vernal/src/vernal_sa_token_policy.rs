//! Sa-Token-Rust 面向 Vernal 操作的授权策略对象。

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use sa_token_core::SaTokenManager;
use vernal_aop::Operation;
use vernal_web::SecurityPrincipal;

use crate::VernalSaTokenError;

type Requirements = HashMap<Operation, Arc<[Arc<str>]>>;

/// 按 Vernal `Operation` 声明角色与权限要求的不可变授权策略。
///
/// 策略由 Sa-Token-Rust 消费侧拥有，因此 Vernal 内核不解释角色、权限或通配符。
/// Builder 方法消费并返回 `Self`；应用在完成声明后用 `Arc` 共享给 IoC 容器与
/// 认证拦截器，运行期只有只读 HashMap 查找，不需要全局注册表或写锁。
#[derive(Clone, Debug, Default)]
pub struct VernalSaTokenPolicy {
    all_roles: Requirements,
    any_roles: Requirements,
    all_permissions: Requirements,
    any_permissions: Requirements,
}

impl VernalSaTokenPolicy {
    /// 创建不限制任何操作的空策略。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回进程内可共享的空策略。
    ///
    /// 默认组件包复用同一个空策略，避免每次创建认证组件时产生无意义分配。
    #[must_use]
    pub fn empty_shared() -> Arc<Self> {
        static EMPTY: OnceLock<Arc<VernalSaTokenPolicy>> = OnceLock::new();
        Arc::clone(EMPTY.get_or_init(|| Arc::new(Self::new())))
    }

    /// 声明操作必须同时具有全部指定角色。
    ///
    /// 空列表仍会被记录，并在运行期按 fail-closed 语义拒绝访问，防止配置计算
    /// 错误意外放开受保护操作。
    #[must_use]
    pub fn require_all_roles<I, S>(mut self, operation: Operation, roles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Arc<str>>,
    {
        self.all_roles
            .insert(operation, collect_requirements(roles));
        self
    }

    /// 声明操作至少具有一个指定角色。
    ///
    /// 空列表按 fail-closed 语义处理。
    #[must_use]
    pub fn require_any_role<I, S>(mut self, operation: Operation, roles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Arc<str>>,
    {
        self.any_roles
            .insert(operation, collect_requirements(roles));
        self
    }

    /// 声明操作必须同时具有全部指定权限。
    ///
    /// 权限匹配与 Sa-Token-Rust `StpUtil::has_permission` 保持一致：支持精确值、
    /// 全局 `*` 以及 `orders:*` 一类前缀通配符。空列表按 fail-closed 处理。
    #[must_use]
    pub fn require_all_permissions<I, S>(mut self, operation: Operation, permissions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Arc<str>>,
    {
        self.all_permissions
            .insert(operation, collect_requirements(permissions));
        self
    }

    /// 声明操作至少具有一个指定权限。
    ///
    /// 空列表按 fail-closed 语义处理。
    #[must_use]
    pub fn require_any_permission<I, S>(mut self, operation: Operation, permissions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Arc<str>>,
    {
        self.any_permissions
            .insert(operation, collect_requirements(permissions));
        self
    }

    /// 返回指定操作是否具有至少一项授权要求。
    #[must_use]
    pub fn protects(&self, operation: &Operation) -> bool {
        self.all_roles.contains_key(operation)
            || self.any_roles.contains_key(operation)
            || self.all_permissions.contains_key(operation)
            || self.any_permissions.contains_key(operation)
    }

    /// 返回策略是否没有保护任何操作。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.all_roles.is_empty()
            && self.any_roles.is_empty()
            && self.all_permissions.is_empty()
            && self.any_permissions.is_empty()
    }

    /// 对已经完成认证的主体执行操作级授权。
    ///
    /// 没有规则的操作直接通过；有规则但没有主体时返回 401；角色或权限不足返回
    /// 403。只有存在权限规则时才读取权限数据，底层存储错误不会伪装成“无权限”，
    /// 而是保留为结构化基础设施失败。
    ///
    /// # Errors
    ///
    /// 操作需要登录、主体不满足规则，或 Sa-Token 权限数据读取失败时返回
    /// [`VernalSaTokenError`]。
    pub async fn authorize(
        &self,
        operation: &Operation,
        manager: &SaTokenManager,
        principal: Option<&SecurityPrincipal>,
    ) -> Result<(), VernalSaTokenError> {
        if !self.protects(operation) {
            return Ok(());
        }
        let principal = principal.ok_or(VernalSaTokenError::Unauthorized)?;

        // 角色已经由认证 Bridge 写入只读主体，不重复访问 Sa-Token 存储。
        if let Some(required) = self.all_roles.get(operation) {
            ensure_all(required, |role| principal.has_role(role))?;
        }
        if let Some(required) = self.any_roles.get(operation) {
            ensure_any(required, |role| principal.has_role(role))?;
        }

        // 权限只有在策略实际声明时才读取一次，并复用同一份快照完成 AND/OR 检查。
        let needs_permissions = self.all_permissions.contains_key(operation)
            || self.any_permissions.contains_key(operation);
        if needs_permissions {
            let granted = manager.get_permissions(principal.subject()).await?;
            if let Some(required) = self.all_permissions.get(operation) {
                ensure_all(required, |permission| {
                    permission_is_granted(&granted, permission)
                })?;
            }
            if let Some(required) = self.any_permissions.get(operation) {
                ensure_any(required, |permission| {
                    permission_is_granted(&granted, permission)
                })?;
            }
        }

        Ok(())
    }
}

/// 把调用方声明转换为便于并发共享的只读切片。
fn collect_requirements<I, S>(requirements: I) -> Arc<[Arc<str>]>
where
    I: IntoIterator<Item = S>,
    S: Into<Arc<str>>,
{
    requirements.into_iter().map(Into::into).collect()
}

/// 校验全部要求；空要求也拒绝，以保持安全配置 fail-closed。
fn ensure_all(
    requirements: &[Arc<str>],
    predicate: impl Fn(&str) -> bool,
) -> Result<(), VernalSaTokenError> {
    if !requirements.is_empty()
        && requirements
            .iter()
            .all(|requirement| predicate(requirement))
    {
        return Ok(());
    }
    Err(VernalSaTokenError::Forbidden)
}

/// 校验任一要求；空要求也拒绝，以保持安全配置 fail-closed。
fn ensure_any(
    requirements: &[Arc<str>],
    predicate: impl Fn(&str) -> bool,
) -> Result<(), VernalSaTokenError> {
    if requirements
        .iter()
        .any(|requirement| predicate(requirement))
    {
        return Ok(());
    }
    Err(VernalSaTokenError::Forbidden)
}

/// 按 Sa-Token-Rust 既有规则匹配一个权限。
fn permission_is_granted(granted: &[String], required: &str) -> bool {
    granted.iter().any(|permission| {
        permission == required
            || permission == "*"
            || permission
                .strip_suffix(":*")
                .is_some_and(|prefix| required.starts_with(prefix))
    })
}
