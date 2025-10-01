




```bash
cargo run create-token \
    --private-key-path ~/.config/bsc/four_meme_test.txt \
    --name balana \
    --short-name blll \
    --description "nanana" \
    --img-url "https://static.four.meme/market/406e4678-9da5-4cff-8da0-8f40fd1874891198309339510750538.png" \
    --label AI
```



```bash
cargo run buy-token \
    --private-key-path ~/.config/bsc/four_meme_test.txt \
    --token 0x143a49227f68ce28633724be1b07a0f8e4f34444 \
    --min-amount 100 \
    --funds 1000000000000
```



```bash
cargo run sell-token \
    --private-key-path ~/.config/bsc/four_meme_test.txt \
    --token 0x143a49227f68ce28633724be1b07a0f8e4f34444 \
    --amount 10000000000
```