//! Sa-Token Vernal 认证切点对象。

use vernal_aop::{Operation, Pointcut};

/// 让 Sa-Token 认证流覆盖应用声明的全部 Vernal 操作。
///
/// 是否需要登录仍由 Sa-Token `PathAuthConfig` 根据 owned HTTP 快照决定；切点不
/// 复制路径规则。这样匿名路由也能完成“有 Token 则校验”的上下文建立，同时
/// 受保护路由保持 Sa-Token 原有的拒绝语义。
#[derive(Clone, Copy, Debug, Default)]
pub struct VernalSaTokenPointcut;

impl Pointcut for VernalSaTokenPointcut {
    fn matches(&self, _operation: &Operation) -> bool {
        true
    }
}
