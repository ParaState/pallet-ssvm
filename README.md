# Pellet SSVM

SSVM pallet is as a runtime library for [Substrate](https://substrate.dev/docs/en/conceptual/runtime/frame)

At the first stage we add a dependency crate [rust-ssvm](https://github.com/second-state/rust-ssvm) as our ewasm engine.

## Troubleshooting
We use git repository as `crate rust-ssvm` path. So we need update file as below before we run `cargo build`.

- `~/.cargo/config`

```
[net]
git-fetch-with-cli = true
```

## License
Pellet SSVM is [AGPL 3.0 licensed](LICENSE).