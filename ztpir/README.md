## Dev Environment Setup

### Prerequisites

```shell
# generally required development tooling
mise install
mise run setup
```

### Debugging

1. container ports don't publish to localhost
    * https://github.com/apple/container/issues/1702
    * System Settings → Privacy & Security → Local Network → container-runtime-linux should be enabled
    * ```shell
      container stop --all
      container delete --all
      container system stop
      container system start
      ```
    * this is because container-runtime-linux uses 192.168 instead of any localhost addresses, which will also likely 
      break some people's networking on LANs that overlap with the selected ip or that use the entire 192.168 address 
      space since it's explicitly reserved for private networks and NOT local networks.  Apple fucked up badly here.