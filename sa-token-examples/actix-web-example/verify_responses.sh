#!/bin/bash

# 详细验证每个接口的返回值是否符合预期
# Detailed verification of each endpoint's response values

BASE_URL="http://localhost:3000"
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PASSED=0
FAILED=0

verify_json() {
    local name=$1
    local json=$2
    local field=$3
    local expected=$4
    
    local actual=$(echo "$json" | jq -r "$field" 2>/dev/null)
    
    if [ "$actual" = "$expected" ]; then
        echo -e "    ${GREEN}✓${NC} $name: $field = '$expected'"
        ((PASSED++))
        return 0
    else
        echo -e "    ${RED}✗${NC} $name: $field 期望 '$expected', 实际 '$actual'"
        ((FAILED++))
        return 1
    fi
}

verify_json_contains() {
    local name=$1
    local json=$2
    local field=$3
    local expected=$4
    
    local actual=$(echo "$json" | jq -r "$field" 2>/dev/null)
    
    if echo "$actual" | grep -q "$expected"; then
        echo -e "    ${GREEN}✓${NC} $name: $field 包含 '$expected'"
        ((PASSED++))
        return 0
    else
        echo -e "    ${RED}✗${NC} $name: $field 不包含 '$expected' (实际: '$actual')"
        ((FAILED++))
        return 1
    fi
}

echo "=========================================="
echo "详细响应值验证"
echo "=========================================="
echo ""

# 1. 登录获取Token
echo -e "${BLUE}=== 1. 登录并获取Token ===${NC}"
admin_response=$(curl -s -X POST -H "Content-Type: application/json" -d '{"username":"admin","password":"admin123"}' "$BASE_URL/api/login")
admin_token=$(echo "$admin_response" | jq -r '.data.token' 2>/dev/null)
verify_json "Admin登录" "$admin_response" ".code" "0"
verify_json "Admin登录" "$admin_response" ".message" "success"
verify_json_contains "Admin登录" "$admin_response" ".data.user_info.username" "admin"
verify_json_contains "Admin登录" "$admin_response" ".data.user_info.nickname" "管理员"

user_response=$(curl -s -X POST -H "Content-Type: application/json" -d '{"username":"user","password":"user123"}' "$BASE_URL/api/login")
user_token=$(echo "$user_response" | jq -r '.data.token' 2>/dev/null)
verify_json "User登录" "$user_response" ".code" "0"

guest_response=$(curl -s -X POST -H "Content-Type: application/json" -d '{"username":"guest","password":"guest123"}' "$BASE_URL/api/login")
guest_token=$(echo "$guest_response" | jq -r '.data.token' 2>/dev/null)
verify_json "Guest登录" "$guest_response" ".code" "0"
echo ""

# 2. 用户信息验证
echo -e "${BLUE}=== 2. 用户信息接口验证 ===${NC}"
user_info=$(curl -s -H "Authorization: $admin_token" "$BASE_URL/api/user/info")
verify_json "用户信息" "$user_info" ".code" "0"
verify_json "用户信息" "$user_info" ".data.id" "admin"
verify_json "用户信息" "$user_info" ".data.username" "admin"
verify_json "用户信息" "$user_info" ".data.nickname" "管理员"
verify_json_contains "用户信息" "$user_info" ".data.email" "admin@example.com"
echo ""

# 3. 权限检查验证
echo -e "${BLUE}=== 3. 权限检查验证 ===${NC}"
# Admin有user:list权限
user_list=$(curl -s -H "Authorization: $admin_token" "$BASE_URL/api/user/list")
verify_json "用户列表(Admin)" "$user_list" ".code" "0"
verify_json "用户列表(Admin)" "$user_list" ".data | length" "2"

# User有user:list权限
user_list_user=$(curl -s -H "Authorization: $user_token" "$BASE_URL/api/user/list")
verify_json "用户列表(User)" "$user_list_user" ".code" "0"

# Guest无user:list权限
user_list_guest=$(curl -s -H "Authorization: $guest_token" "$BASE_URL/api/user/list")
verify_json "用户列表(Guest)" "$user_list_guest" ".code" "403"
verify_json_contains "用户列表(Guest)" "$user_list_guest" ".message" "Permission denied"
echo ""

# 4. 角色检查验证
echo -e "${BLUE}=== 4. 角色检查验证 ===${NC}"
admin_panel=$(curl -s -H "Authorization: $admin_token" "$BASE_URL/api/admin/panel")
verify_json "管理员面板(Admin)" "$admin_panel" ".code" "0"
verify_json "管理员面板(Admin)" "$admin_panel" ".data" "管理员面板"

admin_panel_user=$(curl -s -H "Authorization: $user_token" "$BASE_URL/api/admin/panel")
verify_json "管理员面板(User)" "$admin_panel_user" ".code" "403"
verify_json_contains "管理员面板(User)" "$admin_panel_user" ".message" "Role denied"

admin_stats=$(curl -s -H "Authorization: $admin_token" "$BASE_URL/api/admin/stats")
verify_json "管理员统计" "$admin_stats" ".code" "0"
verify_json "管理员统计" "$admin_stats" ".data.total_users" "100"
verify_json "管理员统计" "$admin_stats" ".data.active_users" "80"
verify_json "管理员统计" "$admin_stats" ".data.new_users_today" "5"
echo ""

# 5. 多权限检查验证
echo -e "${BLUE}=== 5. 多权限检查验证 ===${NC}"
manage_user=$(curl -s -X POST -H "Content-Type: application/json" -H "Authorization: $admin_token" -d '{"user_id":"123","action":"update"}' "$BASE_URL/api/user/manage")
verify_json "管理用户(Admin)" "$manage_user" ".code" "0"
verify_json_contains "管理用户(Admin)" "$manage_user" ".data" "管理成功"

manage_user_user=$(curl -s -X POST -H "Content-Type: application/json" -H "Authorization: $user_token" -d '{"user_id":"123","action":"update"}' "$BASE_URL/api/user/manage")
verify_json "管理用户(User)" "$manage_user_user" ".code" "403"
verify_json_contains "管理用户(User)" "$manage_user_user" ".message" "Permission denied"
echo ""

# 6. 权限管理验证
echo -e "${BLUE}=== 6. 权限管理验证 ===${NC}"
permission_list=$(curl -s -H "Authorization: $admin_token" "$BASE_URL/api/permission/list")
verify_json "权限列表" "$permission_list" ".code" "0"
verify_json_contains "权限列表" "$permission_list" ".data.admin" "user:list"
verify_json_contains "权限列表" "$permission_list" ".data.user" "user:list"

# 添加权限
add_perm=$(curl -s -X POST -H "Content-Type: application/json" -H "Authorization: $admin_token" -d '{"user_id":"user","permission":"user:test"}' "$BASE_URL/api/permission/add")
verify_json "添加权限" "$add_perm" ".code" "0"
verify_json_contains "添加权限" "$add_perm" ".data" "添加权限"

# 移除权限
remove_perm=$(curl -s -X POST -H "Content-Type: application/json" -H "Authorization: $admin_token" -d '{"user_id":"user","permission":"user:test"}' "$BASE_URL/api/permission/remove")
verify_json "移除权限" "$remove_perm" ".code" "0"
verify_json_contains "移除权限" "$remove_perm" ".data" "成功移除"

role_list=$(curl -s -H "Authorization: $admin_token" "$BASE_URL/api/role/list")
verify_json "角色列表" "$role_list" ".code" "0"
verify_json_contains "角色列表" "$role_list" ".data.admin" "admin"
verify_json_contains "角色列表" "$role_list" ".data.user" "user"
echo ""

# 7. 错误处理验证
echo -e "${BLUE}=== 7. 错误处理验证 ===${NC}"
# 无Token访问
no_token=$(curl -s "$BASE_URL/api/user/info")
verify_json "无Token访问" "$no_token" ".code" "401"
verify_json_contains "无Token访问" "$no_token" ".message" "Authentication error"

# 错误密码
wrong_pass=$(curl -s -X POST -H "Content-Type: application/json" -d '{"username":"admin","password":"wrong"}' "$BASE_URL/api/login")
verify_json "错误密码" "$wrong_pass" ".code" "401"
verify_json_contains "错误密码" "$wrong_pass" ".message" "用户名或密码错误"
echo ""

# 8. 公开接口验证
echo -e "${BLUE}=== 8. 公开接口验证 ===${NC}"
health=$(curl -s "$BASE_URL/api/health")
verify_json "健康检查" "$health" ".status" "ok"
verify_json "健康检查" "$health" ".service" "sa-token-rust"
verify_json "健康检查" "$health" ".version" "0.1.0"

index=$(curl -s "$BASE_URL/")
if echo "$index" | grep -q "Welcome"; then
    echo -e "    ${GREEN}✓${NC} 首页: 包含 'Welcome'"
    ((PASSED++))
else
    echo -e "    ${RED}✗${NC} 首页: 不包含 'Welcome' (实际: '$index')"
    ((FAILED++))
fi
echo ""

# 总结
echo "=========================================="
echo "详细验证总结"
echo "=========================================="
echo -e "通过: ${GREEN}$PASSED${NC}"
echo -e "失败: ${RED}$FAILED${NC}"
echo "总计: $((PASSED + FAILED))"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ 所有响应值验证通过！${NC}"
    exit 0
else
    echo -e "${RED}✗ 有 $FAILED 个验证失败${NC}"
    exit 1
fi

