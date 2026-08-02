#!/bin/bash
# 测试 Actix-web 示例

# 默认使用内存存储
if [ "$1" == "redis" ]; then
    echo "使用 Redis 存储运行示例..."
    # 使用 Redis 特性编译并运行
    cargo run --features redis
elif [ "$1" == "demo" ]; then
    echo "开启 StpUtil 演示..."
    # 开启 StpUtil 演示
    DEMO_STP_UTIL=1 cargo run
elif [ "$1" == "redis-demo" ]; then
    echo "使用 Redis 存储并开启 StpUtil 演示..."
    # 使用 Redis 特性并开启 StpUtil 演示
    DEMO_STP_UTIL=1 cargo run --features redis
else
    echo "使用内存存储运行示例..."
    # 默认使用内存存储
    cargo run
fi

# 使用方法:
# ./test.sh         - 使用内存存储
# ./test.sh redis   - 使用Redis存储
# ./test.sh demo    - 开启StpUtil演示
# ./test.sh redis-demo - 使用Redis存储并开启StpUtil演示
