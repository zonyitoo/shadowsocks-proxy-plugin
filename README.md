# shadowsocks Proxy Plugin

A shadowsocks SIP003 Plugin for local service to connect remote server through a Proxy.

```plain
+---------+       +---------+
| CLIENTS |-----> | sslocal |
+---------+       +---------+
                       |
                       v
                  +--------------+                      +---------------+       +----------+
                  | Proxy Plugin |----HTTP/SOCKS5/...-->| Proxy Service |------>| ssserver |
                  +--------------+                      +---------------+       +----------+
```

## Usage

Use it with a shadowsocks local service (sslocal) as a plugin.

```jsonc
{
    "local_address": "127.0.0.1",
    "local_port": 1080,
    "plugin": "proxy-plugin-local",
    "plugin_opts": "proxy_protocol=http&proxy_addr=127.0.0.1:3125"
}
```

Plugin Options:

| Field Name | Required | Type | Explaination |
| ---------- | -------- | ---- | ------------ |
| `proxy_protocol` | Yes | String | The Protocol of Proxy. `socks5`, `http` |
| `proxy_addr` | Yes | String | The address of Proxy. `IP:Port` |
| `outbound_fwmark` | No | Integer | Set `SO_MARK` for outbound sockets |
| `outbound_user_cookie` | No | Integer | Set `SO_USER_COOKIE` for outbound sockets |
| `outbound_bind_interface` | No | String | Set the interface name for outbound sockets to bind with |
| `outbound_bind_addr` | No | String | Set the address for outbound sockets to `bind()` |
| `tcp_keep_alive` | No | Boolean | Set to `true` to enable TCP Keep Alive |
| `tcp_fast_open` | No | Boolean | Set to `true` to enable TCP Fast Open |
| `mptcp` | No | Boolean | Set to `true` to enable Multipart-TCP |
| `ipv6_first` | No | Boolean | Set to `true` to enable IPv6 first when resolving `proxy_addr` |
