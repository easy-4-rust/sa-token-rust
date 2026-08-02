#!/bin/bash

# sa-token-rust Axum 示例测试脚本

BASE_URL="http://localhost:3000"

echo "================================"
echo "sa-token-rust Axum 完整示例测试"
echo "================================"
echo ""

# 1. 健康检查
echo "1️⃣ 测试健康检查..."
curl -s "$BASE_URL/api/health" | jq .
echo ""

# 2. 管理员登录
echo "2️⃣ 管理员登录..."
ADMIN_TOKEN=$(curl -s -X POST "$BASE_URL/api/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}' | jq -r '.data.token')

if [ "$ADMIN_TOKEN" != "null" ] && [ ! -z "$ADMIN_TOKEN" ]; then
    echo "✅ 管理员登录成功！Token: $ADMIN_TOKEN"
else
    echo "❌ 管理员登录失败！"
    exit 1
fi
echo ""

# 3. 普通用户登录
echo "3️⃣ 普通用户登录..."
USER_TOKEN=$(curl -s -X POST "$BASE_URL/api/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"user","password":"user123"}' | jq -r '.data.token')

if [ "$USER_TOKEN" != "null" ] && [ ! -z "$USER_TOKEN" ]; then
    echo "✅ 普通用户登录成功！Token: $USER_TOKEN"
else
    echo "❌ 普通用户登录失败！"
    exit 1
fi
echo ""

# 4. 访客登录
echo "4️⃣ 访客登录..."
GUEST_TOKEN=$(curl -s -X POST "$BASE_URL/api/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"guest","password":"guest123"}' | jq -r '.data.token')

if [ "$GUEST_TOKEN" != "null" ] && [ ! -z "$GUEST_TOKEN" ]; then
    echo "✅ 访客登录成功！Token: $GUEST_TOKEN"
else
    echo "❌ 访客登录失败！"
    exit 1
fi
echo ""

echo "================================"
echo "权限测试"
echo "================================"
echo ""

# 5. 查看权限列表（需要 admin 角色）
echo "5️⃣ 查看所有用户权限（admin 角色）..."
curl -s -X GET "$BASE_URL/api/permission/list" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq .
echo ""

# 6. 查看角色列表（需要 admin 角色）
echo "6️⃣ 查看所有用户角色（admin 角色）..."
curl -s -X GET "$BASE_URL/api/role/list" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq .
echo ""

# 7. 为普通用户添加权限
echo "7️⃣ 为普通用户添加新权限 article:create..."
curl -s -X POST "$BASE_URL/api/permission/add" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"user_id":"user","permission":"article:create"}' | jq .
echo ""

# 8. 再次查看权限列表，确认权限已添加
echo "8️⃣ 确认权限已添加..."
curl -s -X GET "$BASE_URL/api/permission/list" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq .data.user
echo ""

# 9. 移除刚才添加的权限
echo "9️⃣ 移除刚才添加的权限..."
curl -s -X POST "$BASE_URL/api/permission/remove" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"user_id":"user","permission":"article:create"}' | jq .
echo ""

echo "================================"
echo "访问控制测试"
echo "================================"
echo ""

# 10. 管理员访问用户列表（需要 user:list 权限）
echo "🔟 管理员访问用户列表（user:list）..."
curl -s -X GET "$BASE_URL/api/user/list" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq .
echo ""

# 11. 普通用户访问用户列表（需要 user:list 权限）
echo "1️⃣1️⃣ 普通用户访问用户列表（user:list）..."
curl -s -X GET "$BASE_URL/api/user/list" \
  -H "Authorization: Bearer $USER_TOKEN" | jq .
echo ""

# 12. 访客访问用户列表（无 user:list 权限，应该失败）
echo "1️⃣2️⃣ 访客访问用户列表（无权限，应该失败）..."
curl -s -X GET "$BASE_URL/api/user/list" \
  -H "Authorization: Bearer $GUEST_TOKEN" | jq .
echo ""

# 13. 管理员访问管理面板（需要 admin 角色）
echo "1️⃣3️⃣ 管理员访问管理面板（admin 角色）..."
curl -s -X GET "$BASE_URL/api/admin/panel" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq .
echo ""

# 14. 普通用户访问管理面板（无 admin 角色，应该失败）
echo "1️⃣4️⃣ 普通用户访问管理面板（无权限，应该失败）..."
curl -s -X GET "$BASE_URL/api/admin/panel" \
  -H "Authorization: Bearer $USER_TOKEN" | jq .
echo ""

echo "================================"
echo "✅ 测试完成！"
echo "================================"
echo ""
echo "权限标识写入说明："
echo "1. 启动时初始化：在 init_test_permissions() 函数中"
echo "2. 通过 API 动态添加：使用 /api/permission/add 接口"
echo "3. 从数据库加载：在 PermissionService::load_from_database() 中实现"
echo ""

