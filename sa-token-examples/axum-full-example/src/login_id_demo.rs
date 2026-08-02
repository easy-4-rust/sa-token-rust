// Author: 金书记
//
//! LoginId 多类型支持演示

use sa_token_plugin_axum::StpUtil;

/// 演示 LoginId 支持多种类型
pub async fn demo_login_id_types() -> anyhow::Result<()> {
    tracing::info!("🎯 LoginId 多类型支持演示");
    tracing::info!("{}", "=".repeat(60));
    
    // ==================== 1. 字符串类型 ID ====================
    tracing::info!("\n1️⃣ 字符串类型 ID");
    
    let string_id = String::from("user_string");
    let token1 = StpUtil::login(string_id.as_str()).await?;
    tracing::info!(token_length = token1.as_str().len(), "✅ String 类型登录成功，身份与 Token 已脱敏");
    
    let str_id = "user_str";
    let token2 = StpUtil::login(str_id).await?;
    tracing::info!(token_length = token2.as_str().len(), "✅ &str 类型登录成功，身份与 Token 已脱敏");
    
    // ==================== 2. 整数类型 ID ====================
    tracing::info!("\n2️⃣ 整数类型 ID");
    
    // i32
    let i32_id: i32 = 10001;
    let token3 = StpUtil::login(i32_id).await?;
    tracing::info!(token_length = token3.as_str().len(), "✅ i32 类型登录成功，身份与 Token 已脱敏");
    
    // i64
    let i64_id: i64 = 10002_i64;
    let token4 = StpUtil::login(i64_id).await?;
    tracing::info!(token_length = token4.as_str().len(), "✅ i64 类型登录成功，身份与 Token 已脱敏");
    
    // u32
    let u32_id: u32 = 10003;
    let token5 = StpUtil::login(u32_id).await?;
    tracing::info!(token_length = token5.as_str().len(), "✅ u32 类型登录成功，身份与 Token 已脱敏");
    
    // u64
    let u64_id: u64 = 10004_u64;
    let token6 = StpUtil::login(u64_id).await?;
    tracing::info!(token_length = token6.as_str().len(), "✅ u64 类型登录成功，身份与 Token 已脱敏");
    // ==================== 3. 权限和角色管理 ====================
    tracing::info!("\n3️⃣ 权限和角色管理（使用数字 ID）");
    
    // 为数字 ID 用户设置权限
    StpUtil::set_permissions(
        10001,
        vec![
            "order:create".to_string(),
            "order:view".to_string(),
            "order:update".to_string(),
        ],
    ).await?;
    tracing::info!("✅ 为用户 10001 设置权限");
    
    // 为数字 ID 用户设置角色
    StpUtil::set_roles(
        10001,
        vec!["vip".to_string(), "user".to_string()],
    ).await?;
    tracing::info!("✅ 为用户 10001 设置角色");
    
    // 查询权限
    let perms = StpUtil::get_permissions(10001).await;
    tracing::info!("📋 用户 10001 的权限: {:?}", perms);
    
    // 查询角色
    let roles = StpUtil::get_roles(10001).await;
    tracing::info!("📋 用户 10001 的角色: {:?}", roles);
    
    // 检查权限
    let has_perm = StpUtil::has_permission(10001, "order:create").await;
    tracing::info!("🔍 用户 10001 是否有 order:create 权限: {}", has_perm);
    
    // 检查角色
    let has_role = StpUtil::has_role(10001, "vip").await;
    tracing::info!("🔍 用户 10001 是否有 vip 角色: {}", has_role);
    
    // ==================== 4. Session 操作 ====================
    tracing::info!("\n4️⃣ Session 操作（使用数字 ID）");
    
    StpUtil::set_session_value(10001, "nickname", "VIP用户").await?;
    StpUtil::set_session_value(10001, "level", 5).await?;
    tracing::info!("✅ 为用户 10001 设置 Session 值");
    
    let nickname: Option<String> = StpUtil::get_session_value(10001, "nickname").await?;
    let level: Option<i32> = StpUtil::get_session_value(10001, "level").await?;
    tracing::info!("📋 用户 10001 的 Session: nickname={:?}, level={:?}", nickname, level);
    
    // ==================== 5. 混合使用 ====================
    tracing::info!("\n5️⃣ 混合使用不同类型的 ID");
    
    // 为不同类型的 ID 设置权限
    StpUtil::add_permission("user_str", "profile:edit").await?;
    StpUtil::add_permission(10002_i64, "profile:edit").await?;
    StpUtil::add_permission(10003_u32, "profile:edit").await?;
    
    tracing::info!("✅ 为不同类型的 ID 添加权限成功");
    
    // 验证权限
    let has1 = StpUtil::has_permission("user_str", "profile:edit").await;
    let has2 = StpUtil::has_permission(10002_i64, "profile:edit").await;
    let has3 = StpUtil::has_permission(10003_u32, "profile:edit").await;
    
    tracing::info!("🔍 权限验证结果:");
    tracing::info!("   - user_str: {}", has1);
    tracing::info!("   - 10002 (i64): {}", has2);
    tracing::info!("   - 10003 (u32): {}", has3);
    
    // ==================== 6. 批量操作 ====================
    tracing::info!("\n6️⃣ 批量踢人下线");
    
    let ids: Vec<i64> = vec![10001, 10002];
    StpUtil::kick_out_batch(&ids).await?;
    tracing::info!("✅ 批量踢出用户: {:?}", ids);
    
    tracing::info!("\n{}", "=".repeat(60));
    tracing::info!("✅ LoginId 多类型支持演示完成！");
    tracing::info!("\n💡 总结：");
    tracing::info!("   - 支持 String, &str, i32, i64, u32, u64 等类型");
    tracing::info!("   - 内部自动转换为字符串存储");
    tracing::info!("   - 所有 StpUtil 方法都支持多类型 ID");
    tracing::info!("   - 使用时完全透明，无需手动转换\n");
    
    Ok(())
}
