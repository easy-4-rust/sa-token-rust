#!/bin/bash

# Actix-web Example 完整功能测试脚本
# Complete functional test script for Actix-web Example

BASE_URL="http://localhost:3000"
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 计数器
PASSED=0
FAILED=0

# 测试函数
test_endpoint() {
    local name=$1
    local method=$2
    local url=$3
    local data=$4
    local expected_code=$5
    local token=$6
    
    echo -n "测试: $name ... "
    
    if [ -z "$token" ]; then
        if [ "$method" = "GET" ]; then
            response=$(curl -s -w "\n%{http_code}" "$url" 2>/dev/null)
        else
            response=$(curl -s -w "\n%{http_code}" -X "$method" -H "Content-Type: application/json" -d "$data" "$url" 2>/dev/null)
        fi
    else
        if [ "$method" = "GET" ]; then
            response=$(curl -s -w "\n%{http_code}" -H "Authorization: $token" "$url" 2>/dev/null)
        else
            response=$(curl -s -w "\n%{http_code}" -X "$method" -H "Content-Type: application/json" -H "Authorization: $token" -d "$data" "$url" 2>/dev/null)
        fi
    fi
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')
    
    if [ "$http_code" = "$expected_code" ]; then
        echo -e "${GREEN}✓ PASS${NC} (HTTP $http_code)"
        ((PASSED++))
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        return 0
    else
        echo -e "${RED}✗ FAIL${NC} (期望 HTTP $expected_code, 实际 HTTP $http_code)"
        ((FAILED++))
        echo "响应: $body"
        return 1
    fi
}

echo "=========================================="
echo "Actix-web Example 完整功能测试"
echo "=========================================="
echo ""

# 等待服务器启动
echo "等待服务器启动..."
sleep 2

# ==================== 1. 公开接口测试 ====================
echo -e "${YELLOW}=== 1. 公开接口测试（不需要认证）===${NC}"
test_endpoint "首页" "GET" "$BASE_URL/" "" "200"
test_endpoint "健康检查" "GET" "$BASE_URL/api/health" "" "200"
test_endpoint "用户注册" "POST" "$BASE_URL/api/register" '{"username":"test","password":"test123","nickname":"测试用户"}' "200"
echo ""

# ==================== 2. 登录测试 ====================
echo -e "${YELLOW}=== 2. 登录测试 ===${NC}"

# Admin 登录
echo -n "测试: Admin 登录 ... "
admin_response=$(curl -s -X POST -H "Content-Type: application/json" -d '{"username":"admin","password":"admin123"}' "$BASE_URL/api/login" 2>/dev/null)
admin_code=$(curl -s -o /dev/null -w "%{http_code}" -X POST -H "Content-Type: application/json" -d '{"username":"admin","password":"admin123"}' "$BASE_URL/api/login" 2>/dev/null)
if [ "$admin_code" = "200" ]; then
    ADMIN_TOKEN=$(echo "$admin_response" | jq -r '.data.token' 2>/dev/null)
    echo -e "${GREEN}✓ PASS${NC}"
    echo "Token: ${ADMIN_TOKEN:0:50}..."
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC}"
    ((FAILED++))
    ADMIN_TOKEN=""
fi

# User 登录
echo -n "测试: User 登录 ... "
user_response=$(curl -s -X POST -H "Content-Type: application/json" -d '{"username":"user","password":"user123"}' "$BASE_URL/api/login" 2>/dev/null)
user_code=$(curl -s -o /dev/null -w "%{http_code}" -X POST -H "Content-Type: application/json" -d '{"username":"user","password":"user123"}' "$BASE_URL/api/login" 2>/dev/null)
if [ "$user_code" = "200" ]; then
    USER_TOKEN=$(echo "$user_response" | jq -r '.data.token' 2>/dev/null)
    echo -e "${GREEN}✓ PASS${NC}"
    echo "Token: ${USER_TOKEN:0:50}..."
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC}"
    ((FAILED++))
    USER_TOKEN=""
fi

# Guest 登录
echo -n "测试: Guest 登录 ... "
guest_response=$(curl -s -X POST -H "Content-Type: application/json" -d '{"username":"guest","password":"guest123"}' "$BASE_URL/api/login" 2>/dev/null)
guest_code=$(curl -s -o /dev/null -w "%{http_code}" -X POST -H "Content-Type: application/json" -d '{"username":"guest","password":"guest123"}' "$BASE_URL/api/login" 2>/dev/null)
if [ "$guest_code" = "200" ]; then
    GUEST_TOKEN=$(echo "$guest_response" | jq -r '.data.token' 2>/dev/null)
    echo -e "${GREEN}✓ PASS${NC}"
    echo "Token: ${GUEST_TOKEN:0:50}..."
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL${NC}"
    ((FAILED++))
    GUEST_TOKEN=""
fi

# 错误密码测试
test_endpoint "错误密码登录" "POST" "$BASE_URL/api/login" '{"username":"admin","password":"wrong"}' "401" ""
echo ""

# ==================== 3. 需要登录的接口测试 ====================
echo -e "${YELLOW}=== 3. 需要登录的接口测试 ===${NC}"
test_endpoint "无Token访问用户信息" "GET" "$BASE_URL/api/user/info" "" "401" ""
test_endpoint "Admin获取用户信息" "GET" "$BASE_URL/api/user/info" "" "200" "$ADMIN_TOKEN"
test_endpoint "User获取用户信息" "GET" "$BASE_URL/api/user/info" "" "200" "$USER_TOKEN"
test_endpoint "Admin获取用户资料" "GET" "$BASE_URL/api/user/profile" "" "200" "$ADMIN_TOKEN"
echo ""

# ==================== 4. 权限检查测试 ====================
echo -e "${YELLOW}=== 4. 权限检查测试 ===${NC}"
test_endpoint "Admin访问用户列表（需要user:list权限）" "GET" "$BASE_URL/api/user/list" "" "200" "$ADMIN_TOKEN"
test_endpoint "User访问用户列表（需要user:list权限）" "GET" "$BASE_URL/api/user/list" "" "200" "$USER_TOKEN"
test_endpoint "Guest访问用户列表（无权限）" "GET" "$BASE_URL/api/user/list" "" "403" "$GUEST_TOKEN"
test_endpoint "Admin删除用户（需要user:delete权限）" "POST" "$BASE_URL/api/user/delete" '{"user_id":"123"}' "200" "$ADMIN_TOKEN"
test_endpoint "User删除用户（无权限）" "POST" "$BASE_URL/api/user/delete" '{"user_id":"123"}' "403" "$USER_TOKEN"
echo ""

# ==================== 5. 角色检查测试 ====================
echo -e "${YELLOW}=== 5. 角色检查测试 ===${NC}"
test_endpoint "Admin访问管理员面板（需要admin角色）" "GET" "$BASE_URL/api/admin/panel" "" "200" "$ADMIN_TOKEN"
test_endpoint "User访问管理员面板（无admin角色）" "GET" "$BASE_URL/api/admin/panel" "" "403" "$USER_TOKEN"
test_endpoint "Guest访问管理员面板（无admin角色）" "GET" "$BASE_URL/api/admin/panel" "" "403" "$GUEST_TOKEN"
test_endpoint "Admin访问管理员统计（需要admin角色）" "GET" "$BASE_URL/api/admin/stats" "" "200" "$ADMIN_TOKEN"
echo ""

# ==================== 6. 多权限检查测试 ====================
echo -e "${YELLOW}=== 6. 多权限检查测试 ===${NC}"
test_endpoint "Admin管理用户（需要user:read AND user:write）" "POST" "$BASE_URL/api/user/manage" '{"user_id":"123","action":"update"}' "200" "$ADMIN_TOKEN"
test_endpoint "User管理用户（权限不足）" "POST" "$BASE_URL/api/user/manage" '{"user_id":"123","action":"update"}' "403" "$USER_TOKEN"
echo ""

# ==================== 7. 权限管理接口测试 ====================
echo -e "${YELLOW}=== 7. 权限管理接口测试（需要admin角色）===${NC}"
test_endpoint "Admin查询权限列表" "GET" "$BASE_URL/api/permission/list" "" "200" "$ADMIN_TOKEN"
test_endpoint "User查询权限列表（无admin角色）" "GET" "$BASE_URL/api/permission/list" "" "403" "$USER_TOKEN"
test_endpoint "Admin添加权限" "POST" "$BASE_URL/api/permission/add" '{"user_id":"user","permission":"user:test"}' "200" "$ADMIN_TOKEN"
test_endpoint "Admin移除权限" "POST" "$BASE_URL/api/permission/remove" '{"user_id":"user","permission":"user:test"}' "200" "$ADMIN_TOKEN"
test_endpoint "Admin查询角色列表" "GET" "$BASE_URL/api/role/list" "" "200" "$ADMIN_TOKEN"
echo ""

# ==================== 8. StpUtil演示接口测试 ====================
echo -e "${YELLOW}=== 8. StpUtil演示接口测试 ===${NC}"
test_endpoint "StpUtil演示" "GET" "$BASE_URL/api/demo/stp-util" "" "200" ""
echo ""

# ==================== 测试总结 ====================
echo "=========================================="
echo "测试总结"
echo "=========================================="
echo -e "通过: ${GREEN}$PASSED${NC}"
echo -e "失败: ${RED}$FAILED${NC}"
echo "总计: $((PASSED + FAILED))"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ 所有测试通过！${NC}"
    exit 0
else
    echo -e "${RED}✗ 有 $FAILED 个测试失败${NC}"
    exit 1
fi

