
### apisix 配置
> grpc-transcode需上传proto
``` yaml
{
  "uri": "/v3/satoken/userinfo",
  "name": "sa-token 认证",
  "methods": [
    "GET",
  ],
  "plugins": {
    # 可选
    "cors": {
      "_meta": {
        "disable": false
      },
      "allow_credential": false,
      "allow_headers": "*",
      "allow_methods": "*",
      "allow_origins": "*",
      "expose_headers": "*",
      "max_age": 5
    },
    # http转grpc
    "grpc-transcode": {
      "_meta": {
        "disable": false
      },
      "deadline": 10000,
      "method": "GetUserInfo",
      "proto_id": "sa-token-auth",
      "service": "auth.AuthService"
    },
    # 可选
    "request-id": {
      "_meta": {
        "disable": false
      }
    }
  },
  "upstream": {
    # 上游服务(可主动配置节点或服务发现(k8s,nacos等))
    "nodes": [
      {
        "host": "10.0.0.44",
        "port": 3000,
        "weight": 1
      }
    ],
    "timeout": {
      "connect": 6,
      "send": 6,
      "read": 6
    },
    "type": "roundrobin",
    "scheme": "grpc",
    "pass_host": "pass",
    "keepalive_pool": {
      "idle_timeout": 60,
      "requests": 1000,
      "size": 320
    }
  },
  "status": 1
}

```