// Author: 金书记
//
//! StpUtil 使用演示

use sa_token_plugin_axum::StpUtil;

/// 演示 StpUtil 的各种功能
pub async fn demo_stp_util() -> anyhow::Result<()> {
    tracing::info!("🚀 StpUtil 使用演示");
    tracing::info!("{}", "=".repeat(50));
    
    // 1. 登录
    tracing::info!("\n 1 用户登录");
    let user_id = "demo_user";
    let token = StpUtil::login(user_id).await?;
    tracing::info!("✅ 用户登录成功（身份值不写入日志）");
    tracing::info!(token_length = token.as_str().len(), "token generated; value redacted");
    
    // 2. 检查登录状态
    tracing::info!("\n 2 检查登录状态");
    let is_login = StpUtil::is_login(&token).await;
    tracing::info!("✅ 是否已登录: {}", if is_login { "是" } else { "否" });
    
    // 3. 获取登录 ID
    tracing::info!("\n 3 获取登录 ID");
    let login_id = StpUtil::get_login_id(&token).await?;
    tracing::info!("✅ 当前登录 ID 已解析（值不写入日志）");
    
    // 4. 获取 Token 信息
    tracing::info!("\n 4 获取 Token 信息");
    let token_info = StpUtil::get_token_info(&token).await?;
    tracing::info!("✅ Token 信息:");
    tracing::info!("   - 登录 ID: [REDACTED]");
    tracing::info!("   - 创建时间: {}", token_info.create_time);
    tracing::info!("   - 登录类型: {}", token_info.login_type);
    
    // 5. Session 操作
    tracing::info!("\n 5 Session 操作");
    
    // 设置 Session 值
    StpUtil::set_session_value(&login_id, "username", "演示用户").await?;
    StpUtil::set_session_value(&login_id, "age", 25).await?;
    tracing::info!("✅ 已设置 Session 值");
    
    // 获取 Session 值
    let username: Option<String> = StpUtil::get_session_value(&login_id, "username").await?;
    let age: Option<i32> = StpUtil::get_session_value(&login_id, "age").await?;
    tracing::info!("✅ Session 值:");
    tracing::info!("   - username: {:?}", username);
    tracing::info!("   - age: {:?}", age);
    
    // 6. Token 有效期
    tracing::info!("\n 6 Token 有效期");
    if let Some(timeout) = StpUtil::get_token_timeout(&token).await? {
        tracing::info!("✅ Token 剩余有效时间: {} 秒", timeout);
        tracing::info!("   约 {} 小时", timeout / 3600);
    } else {
        tracing::info!("✅ Token 永久有效");
    }
    
    // 7. 续期 Token
    tracing::info!("\n 7 续期 Token");
    StpUtil::renew_timeout(&token, 3600).await?;
    tracing::info!("✅ Token 已续期至 1 小时");
    
    if let Some(timeout) = StpUtil::get_token_timeout(&token).await? {
        tracing::info!("   新的剩余时间: {} 秒", timeout);
    }
    
    // 8. 验证登录
    tracing::info!("\n 8 验证登录");
    match StpUtil::check_login(&token).await {
        Ok(_) => tracing::info!("✅ 登录验证通过"),
        Err(e) => tracing::error!("❌ 登录验证失败: {}", e),
    }
    
    // 9. 登出
    tracing::info!("\n 9 登出");
    StpUtil::logout(&token).await?;
    tracing::info!("✅ 用户已登出");
    
    // 10. 再次检查登录状态
    let is_login_after = StpUtil::is_login(&token).await;
    tracing::info!("✅ 登出后是否已登录: {}", if is_login_after { "是" } else { "否" });
    
    // 11. 尝试验证登录（应该失败）
    tracing::info!("\n 10 验证登录（应该失败）");
    match StpUtil::check_login(&token).await {
        Ok(_) => tracing::info!("✅ 登录验证通过"),
        Err(e) => tracing::info!("❌ 登录验证失败（符合预期）: {}", e),
    }
    
    tracing::info!("\n{}", "=".repeat(50));
    tracing::info!("✅ StpUtil 演示完成！");
    
    Ok(())
}
